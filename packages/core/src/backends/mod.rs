pub mod deepseek;
pub mod openai_compat;
pub mod volcengine;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatParams {
    pub messages: Vec<ChatMessage>,
    pub tools: Vec<ToolDef>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    /// Required by API — always serialized (null for assistant tool_call messages).
    pub content: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// OpenAI-compatible tool definition: `{ type: "function", function: { name, description, parameters } }`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// A single todo item for the TodoUpdate event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub content: String,
    pub status: String, // "pending" | "in_progress" | "completed"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    TextDelta { delta: String },
    /// Emitted when the model outputs a reasoning/thinking block (e.g. DeepSeek-R1 reasoning_content).
    ReasoningDelta { delta: String },
    /// Emitted when the reasoning block is complete (finish_reason reached or content starts).
    ReasoningDone,
    ToolCall { id: String, name: String, arguments: String },
    ToolResult { id: String, content: String },
    ApprovalRequired { tool: String, params: String },
    /// Emitted when the agent calls todo_write — frontend renders a task list.
    TodoUpdate { todos: Vec<TodoItem> },
    /// Emitted when the agent calls propose_plan — frontend shows a plan approval card.
    PlanProposed { title: String, steps: Vec<String>, files_to_modify: Vec<String> },
    /// Emitted before and after each API call with token usage estimates and actuals.
    TokenUsage {
        phase: String,           // "before" | "after"
        estimated: u32,          // heuristic estimate (always present)
        actual: Option<u32>,     // API-reported actual (only for phase="after")
        context_window: u32,     // current model's context window size
        context_remaining: u32,  // context_window - max(estimated, actual)
    },
    /// Emitted when the agent trims older conversation turns to stay within limits.
    ContextTrimmed {
        rounds_removed: u32,       // how many conversation turns were dropped
        tokens_freed: u32,         // estimated tokens released
        current_usage_percent: u32, // post-trim usage percentage (0–100)
        context_window: u32,
    },
    Done { usage: Usage },
    Error { message: String },
}

#[async_trait]
pub trait LlmBackend: Send + Sync {
    fn model(&self) -> &str;
    fn base_url(&self) -> &str;
    fn api_key(&self) -> &str;
    async fn chat(&self, params: ChatParams, tx: mpsc::Sender<StreamEvent>) -> Result<Usage, anyhow::Error>;
}

/// Shared SSE streaming implementation for OpenAI-compatible APIs.
pub(crate) async fn openai_compat_chat(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    model: &str,
    params: ChatParams,
    tx: &mpsc::Sender<StreamEvent>,
) -> Result<Usage, anyhow::Error> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    eprintln!("[xuflow] POST {} | model={} | msgs={} | tools={}", url, model, params.messages.len(), params.tools.len());

    let body = serde_json::json!({
        "model": model,
        "messages": params.messages,
        "tools": params.tools,
        "stream": true,
        "temperature": params.temperature.unwrap_or(0.7),
        "max_tokens": params.max_tokens.unwrap_or(4096),
    });

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        tx.send(StreamEvent::Error {
            message: format!("HTTP {} {}: {}", status.as_u16(), status.canonical_reason().unwrap_or(""), text),
        })
        .await
        .ok();
        return Err(anyhow::anyhow!("HTTP {}: {}", status.as_u16(), text));
    }

    use futures_util::StreamExt;
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut usage = Usage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 };
    let mut tool_calls: std::collections::HashMap<usize, (String, String, String)> = std::collections::HashMap::new();
    let mut in_reasoning = false;

    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(c) => c,
            Err(e) => {
                tx.send(StreamEvent::Error { message: format!("Stream error: {}", e) }).await.ok();
                return Err(anyhow::anyhow!("Stream error: {}", e));
            }
        };

        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(line_end) = buffer.find('\n') {
            let line = buffer[..line_end].trim().to_string();
            buffer = buffer[line_end + 1..].to_string();

            if line.is_empty() {
                continue;
            }

            let data = if line.starts_with("data: ") {
                &line[6..]
            } else if line.starts_with("data:") {
                &line[5..]
            } else {
                continue;
            };

            if data == "[DONE]" {
                break;
            }

            let parsed: serde_json::Value = match serde_json::from_str(data) {
                Ok(v) => v,
                Err(_) => continue,
            };

            // Extract usage from final chunk
            if let Some(u) = parsed.get("usage") {
                usage = Usage {
                    prompt_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    completion_tokens: u.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                };
            }

            let choices = match parsed.get("choices") {
                Some(c) => c,
                None => continue,
            };

            let choice = match choices.get(0) {
                Some(c) => c,
                None => continue,
            };

            let delta = match choice.get("delta") {
                Some(d) => d,
                None => continue,
            };

            // ── Reasoning / thinking content (e.g. DeepSeek-R1 reasoning_content) ──
            // We track whether we're currently inside a reasoning block so we can
            // emit ReasoningDone when the model switches from reasoning to normal text.
            let mut reasoning_delta_opt: Option<&str> = None;
            if let Some(rc) = delta.get("reasoning_content").and_then(|v| v.as_str()) {
                if !rc.is_empty() {
                    reasoning_delta_opt = Some(rc);
                }
            }

            if let Some(rc) = reasoning_delta_opt {
                if !in_reasoning {
                    in_reasoning = true;
                }
                tx.send(StreamEvent::ReasoningDelta { delta: rc.to_string() }).await.ok();
            } else if in_reasoning {
                // reasoning_content disappeared → reasoning phase ended
                in_reasoning = false;
                tx.send(StreamEvent::ReasoningDone).await.ok();
            }

            // Text content delta (only emit when not in reasoning phase)
            if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                if !content.is_empty() {
                    tx.send(StreamEvent::TextDelta { delta: content.to_string() }).await.ok();
                }
            }

            // Tool calls delta
            if let Some(tc_array) = delta.get("tool_calls").and_then(|v| v.as_array()) {
                for tc in tc_array {
                    let index = tc.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                    let id = tc.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let func = tc.get("function");
                    let name = func.and_then(|f| f.get("name").and_then(|v| v.as_str())).unwrap_or("");
                    let arguments = func.and_then(|f| f.get("arguments").and_then(|v| v.as_str())).unwrap_or("");

                    let entry = tool_calls.entry(index).or_insert_with(|| (String::new(), String::new(), String::new()));
                    if !id.is_empty() { entry.0 = id.to_string(); }
                    if !name.is_empty() { entry.1 = name.to_string(); }
                    entry.2.push_str(arguments);
                }
            }

            // Check finish_reason for tool_calls completion (also signal ReasoningDone if still in reasoning)
            if let Some(finish) = choice.get("finish_reason").and_then(|v| v.as_str()) {
                if finish == "tool_calls" || finish == "stop" {
                    if in_reasoning {
                        in_reasoning = false;
                        tx.send(StreamEvent::ReasoningDone).await.ok();
                    }
                    // Emit completed tool calls
                    let mut indices: Vec<usize> = tool_calls.keys().copied().collect();
                    indices.sort();
                    for idx in indices {
                        let (id, name, arguments) = &tool_calls[&idx];
                        if !name.is_empty() {
                            tx.send(StreamEvent::ToolCall {
                                id: id.clone(),
                                name: name.clone(),
                                arguments: arguments.clone(),
                            })
                            .await
                            .ok();
                        }
                    }
                    tool_calls.clear();
                }
            }
        }
    }

    // Final safety: if we ended the stream while still reasoning, emit ReasoningDone
    if in_reasoning {
        tx.send(StreamEvent::ReasoningDone).await.ok();
    }

    tx.send(StreamEvent::Done { usage: usage.clone() }).await.ok();
    Ok(usage)
}

// ---------------------------------------------------------------------------
// Token estimation utilities — character-based heuristics (no tiktoken-rs dep)
// ---------------------------------------------------------------------------

pub mod token_counter {
    use super::ChatMessage;

    /// Per-model configurable token estimation coefficients.
    #[derive(Debug, Clone)]
    pub struct TokenEstimateConfig {
        /// CJK characters ≈ 1.3 tokens each in most subword tokenizers.
        pub cjk_coeff: f64,
        /// Non-CJK (Latin, spaces, punctuation): ~4 chars per token.
        pub non_cjk_coeff: f64,
        /// Structured content (JSON, tool returns): symbol-dense, higher token density.
        pub structured_coeff: f64,
    }

    impl Default for TokenEstimateConfig {
        fn default() -> Self {
            Self {
                cjk_coeff: 1.3,
                non_cjk_coeff: 0.25,
                structured_coeff: 0.5,
            }
        }
    }

    /// Check whether a char falls within CJK / Japanese / Korean Unicode blocks.
    pub fn is_cjk_char(c: char) -> bool {
        ('\u{4E00}'..='\u{9FFF}').contains(&c)   // CJK Unified Ideographs
            || ('\u{3400}'..='\u{4DBF}').contains(&c)  // CJK Extension A
            || ('\u{F900}'..='\u{FAFF}').contains(&c)  // CJK Compatibility
            || ('\u{3040}'..='\u{309F}').contains(&c)  // Hiragana
            || ('\u{30A0}'..='\u{30FF}').contains(&c)  // Katakana
            || ('\u{AC00}'..='\u{D7AF}').contains(&c)  // Hangul Syllables
    }

    /// Estimate token count for a plain text string.
    pub fn estimate_tokens(text: &str, config: &TokenEstimateConfig) -> u32 {
        let total_chars = text.chars().count();
        if total_chars == 0 {
            return 0;
        }
        let cjk_count = text.chars().filter(|c| is_cjk_char(*c)).count();
        let non_cjk = total_chars - cjk_count;
        (cjk_count as f64 * config.cjk_coeff + non_cjk as f64 * config.non_cjk_coeff).ceil() as u32
    }

    /// Check if a JSON value looks like structured / tool-return content.
    pub fn is_structured_content(content: &serde_json::Value) -> bool {
        match content {
            serde_json::Value::String(s) => {
                // Heuristic: if it starts with { or [ and is valid JSON, it's structured
                let trimmed = s.trim();
                (trimmed.starts_with('{') || trimmed.starts_with('['))
                    && serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
            }
            serde_json::Value::Object(_) | serde_json::Value::Array(_) => true,
            _ => false,
        }
    }

    /// Estimate tokens for a single ChatMessage.
    /// Adds per-message overhead (role tag, formatting) plus content tokens.
    pub fn estimate_message_tokens(msg: &ChatMessage, config: &TokenEstimateConfig) -> u32 {
        let mut tokens: u32 = 4; // role marker + JSON framing overhead

        // Estimate content tokens
        if let Some(content) = &msg.content {
            match content {
                serde_json::Value::String(s) => {
                    let coeff = if is_structured_content(content) {
                        config.structured_coeff
                    } else {
                        // Use a blended approach: estimate via the standard method
                        // but check if it's mostly code/symbols
                        let alpha_numeric = s.chars().filter(|c| c.is_alphanumeric()).count();
                        let total = s.chars().count().max(1);
                        if (alpha_numeric as f64 / total as f64) < 0.3 {
                            // Sparse alphanumeric — likely code or structured data
                            config.structured_coeff
                        } else {
                            // Use standard estimation
                            return tokens + estimate_tokens(s, config);
                        }
                    };
                    tokens += (s.chars().count() as f64 * coeff).ceil() as u32;
                }
                serde_json::Value::Array(arr) => {
                    for item in arr {
                        if let Some(s) = item.as_str() {
                            tokens += estimate_tokens(s, config);
                        }
                    }
                }
                _ => {}
            }
        }

        // Estimate tool call argument tokens
        if let Some(tc) = &msg.tool_calls {
            for call in tc {
                if let Some(args) = call
                    .get("function")
                    .and_then(|f| f.get("arguments"))
                    .and_then(|a| a.as_str())
                {
                    tokens += estimate_tokens(args, config);
                }
                tokens += 8; // tool call structure overhead
            }
        }

        tokens
    }

    /// Estimate total tokens for a list of messages.
    pub fn estimate_total_tokens(messages: &[ChatMessage], config: &TokenEstimateConfig) -> u32 {
        messages.iter().map(|m| estimate_message_tokens(m, config)).sum()
    }

    /// Get the default context window size for a model.
    pub fn default_context_window(model_id: &str) -> u32 {
        let lower = model_id.to_lowercase();
        if lower.contains("deepseek") {
            return 128_000;
        }
        if lower.contains("doubao") {
            return 128_000;
        }
        if lower.contains("glm") {
            return 128_000;
        }
        // Conservative default
        128_000
    }
}

#[cfg(test)]
mod tests {
    use super::token_counter::*;
    use super::*;

    #[test]
    fn test_estimate_tokens_english() {
        let config = TokenEstimateConfig::default();
        let tokens = estimate_tokens("Hello world, this is a test.", &config);
        // ~30 chars × 0.25 ≈ 8 tokens
        assert!(tokens >= 6 && tokens <= 12, "got {tokens}");
    }

    #[test]
    fn test_estimate_tokens_chinese() {
        let config = TokenEstimateConfig::default();
        let tokens = estimate_tokens("你好世界这是一个测试", &config);
        // 10 CJK chars × 1.3 ≈ 13 tokens
        assert!(tokens >= 10 && tokens <= 16, "got {tokens}");
    }

    #[test]
    fn test_estimate_tokens_mixed() {
        let config = TokenEstimateConfig::default();
        let tokens = estimate_tokens("Hello 你好 World 世界", &config);
        // 4 CJK + 12 non-CJK ≈ 5.2 + 3 = 9
        assert!(tokens >= 6 && tokens <= 14, "got {tokens}");
    }

    #[test]
    fn test_estimate_tokens_empty() {
        let config = TokenEstimateConfig::default();
        assert_eq!(estimate_tokens("", &config), 0);
    }

    #[test]
    fn test_is_structured_content_json_string() {
        let v = serde_json::Value::String(r#"{"key":"value"}"#.into());
        assert!(is_structured_content(&v));
    }

    #[test]
    fn test_is_structured_content_plain_text() {
        let v = serde_json::Value::String("Hello world".into());
        assert!(!is_structured_content(&v));
    }

    #[test]
    fn test_estimate_message_tokens() {
        let config = TokenEstimateConfig::default();
        let msg = ChatMessage {
            role: "user".into(),
            content: Some(serde_json::Value::String("Hello".into())),
            tool_calls: None,
            tool_call_id: None,
        };
        let tokens = estimate_message_tokens(&msg, &config);
        // 4 overhead + 5 chars × 0.25 ≈ 5.25 → 6 ceil
        assert!(tokens >= 5 && tokens <= 10, "got {tokens}");
    }

    #[test]
    fn test_default_context_window() {
        assert_eq!(default_context_window("deepseek-v4-pro"), 128_000);
        assert_eq!(default_context_window("doubao-seed-2.0-code"), 128_000);
        assert_eq!(default_context_window("unknown-model"), 128_000);
    }

    #[test]
    fn test_stream_event_serialization() {
        let ev = StreamEvent::TokenUsage {
            phase: "before".into(),
            estimated: 1000,
            actual: None,
            context_window: 128000,
            context_remaining: 127000,
        };
        let json = serde_json::to_string(&ev).unwrap();
        assert!(json.contains("\"type\":\"TokenUsage\""));
        assert!(json.contains("\"phase\":\"before\""));
        assert!(json.contains("\"estimated\":1000"));
    }
}
