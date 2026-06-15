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
    pub content: serde_json::Value,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    TextDelta { delta: String },
    ToolCall { id: String, name: String, arguments: String },
    ToolResult { id: String, content: String },
    ApprovalRequired { tool: String, params: String },
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

            // Text content delta
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

            // Check finish_reason for tool_calls completion
            if let Some(finish) = choice.get("finish_reason").and_then(|v| v.as_str()) {
                if finish == "tool_calls" || finish == "stop" {
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

    tx.send(StreamEvent::Done { usage: usage.clone() }).await.ok();
    Ok(usage)
}
