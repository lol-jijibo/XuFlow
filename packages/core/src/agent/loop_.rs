/// Agent loop - the core orchestration logic.
///
/// Flow:
///   用户消息 -> 构建 messages[] -> backend.chat(stream) ->
///     ├─ text_delta -> 推送到前端流式显示
///     ├─ tool_use -> 检查危险工具 -> 发送审批事件到前端
///     ├─ tool_result -> 执行结果追加到 messages -> 继续循环
///     └─ done -> 结束本轮
///
/// Context management:
///   - Token estimation via char-based heuristics (configurable per model)
///   - Dynamic turn-based trimming when usage exceeds 80% of context_window
///   - Preserves last N user turns; releases tokens until usage drops below 60%

use crate::agent::types::ApprovalHandler;
use crate::backends::token_counter::{self, TokenEstimateConfig};
use crate::backends::{ChatMessage, ChatParams, FunctionDef, LlmBackend, StreamEvent, ToolDef, Usage};
use crate::tools::ToolRegistry;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;

const MAX_TOOL_ROUNDS: usize = 30;
const DEFAULT_MIN_USER_TURNS: u32 = 3;

pub struct AgentLoop {
    messages: Vec<ChatMessage>,
    backend: Arc<dyn LlmBackend>,
    tools: Arc<ToolRegistry>,
    approval_handler: Arc<dyn ApprovalHandler>,
    /// Max tokens allowed in the context window.
    context_window: u32,
    /// Minimum user turns to preserve during trimming.
    min_user_turns: u32,
    /// Token estimation coefficients (model-configurable).
    token_config: TokenEstimateConfig,
}

impl AgentLoop {
    pub fn new(
        backend: Arc<dyn LlmBackend>,
        tools: Arc<ToolRegistry>,
        approval_handler: Arc<dyn ApprovalHandler>,
    ) -> Self {
        let model = backend.model();
        let default_ctx = token_counter::default_context_window(model);
        Self {
            messages: Vec::new(),
            backend,
            tools,
            approval_handler,
            context_window: default_ctx,
            min_user_turns: DEFAULT_MIN_USER_TURNS,
            token_config: TokenEstimateConfig::default(),
        }
    }

    /// Expose the backend for standalone operations (e.g. title summarization).
    pub fn backend(&self) -> &Arc<dyn LlmBackend> {
        &self.backend
    }

    pub fn with_system_prompt(mut self, prompt: &str) -> Self {
        self.messages.push(ChatMessage {
            role: "system".into(),
            content: Some(Value::String(prompt.into())),
            tool_calls: None,
            tool_call_id: None,
        });
        self
    }

    // ── Context window configuration ──────────────────────────────────

    /// Set a custom context window size (overrides the model default).
    pub fn set_context_window(&mut self, window: u32) {
        self.context_window = window;
    }

    /// Current context window size.
    pub fn context_window(&self) -> u32 {
        self.context_window
    }

    /// Set minimum user turns to preserve during trimming.
    pub fn set_min_user_turns(&mut self, n: u32) {
        self.min_user_turns = n.max(1);
    }

    /// Current minimum user turns.
    pub fn min_user_turns(&self) -> u32 {
        self.min_user_turns
    }

    // ── Token estimation ──────────────────────────────────────────────

    /// Estimate total tokens for all messages currently in context.
    fn estimate_total_tokens(&self) -> u32 {
        token_counter::estimate_total_tokens(&self.messages, &self.token_config)
    }

    // ── Dynamic context trimming ──────────────────────────────────────

    /// Trim older conversation turns to stay within the context window.
    ///
    /// Algorithm:
    ///   1. If estimated tokens < 80% of context_window, do nothing.
    ///   2. Group messages into atomic "turns" (user msg + all following
    ///      assistant/tool msgs until the next user msg).
    ///   3. From the end, mark the last `min_user_turns` turns as protected.
    ///   4. Drop oldest unprotected turns one by one, re-estimating after
    ///      each drop, until usage < 60% or only protected turns remain.
    ///   5. Emit a ContextTrimmed event; do NOT insert a visible system msg.
    fn trim_context(&mut self, tx: &mpsc::Sender<StreamEvent>) {
        let estimated = self.estimate_total_tokens();
        let threshold = self.context_window.saturating_mul(80) / 100;

        if estimated < threshold {
            return; // plenty of headroom
        }

        // Separate system messages (always preserved) from user/assistant/tool messages
        let start_idx = self
            .messages
            .iter()
            .position(|m| m.role != "system")
            .unwrap_or(0);
        let system_msgs: Vec<ChatMessage> = self.messages[..start_idx].to_vec();
        let rest: Vec<ChatMessage> = self.messages[start_idx..].to_vec();

        if rest.is_empty() {
            return;
        }

        // Build atomic turns
        let mut turns: Vec<Vec<ChatMessage>> = Vec::new();
        let mut current: Vec<ChatMessage> = Vec::new();

        for msg in rest {
            if msg.role == "user" && !current.is_empty() {
                turns.push(std::mem::take(&mut current));
            }
            current.push(msg);
        }
        if !current.is_empty() {
            turns.push(current);
        }

        if turns.len() <= self.min_user_turns as usize {
            return; // nothing we can safely remove
        }

        let protected_start = turns.len().saturating_sub(self.min_user_turns as usize);
        let mut removed_count: u32 = 0;
        let tokens_before = token_counter::estimate_total_tokens(
            &turns.iter().flatten().cloned().collect::<Vec<_>>(),
            &self.token_config,
        );

        // Drop oldest unprotected turns until we're below 60% or out of candidates
        let target = self.context_window.saturating_mul(60) / 100;
        let mut drop_idx = 0;

        while drop_idx < protected_start {
            turns.remove(0);
            removed_count += 1;
            drop_idx = drop_idx.saturating_sub(1); // adjust since we removed from front
            // Actually, since we're always removing index 0, the protected_start shifts
            // But we need to recalculate what's protected
            let new_protected_start = turns.len().saturating_sub(self.min_user_turns as usize);
            if new_protected_start == 0 {
                break; // only protected turns remain
            }

            let current_tokens = token_counter::estimate_total_tokens(
                &turns.iter().flatten().cloned().collect::<Vec<_>>(),
                &self.token_config,
            );
            if current_tokens < target {
                break;
            }
            // Keep drop_idx at 0 since we're always removing from front
        }

        // Rebuild messages: system msgs + remaining turns
        let mut new_messages = system_msgs;
        for turn in turns {
            new_messages.extend(turn);
        }

        let tokens_after = token_counter::estimate_total_tokens(&new_messages, &self.token_config);
        let tokens_freed = tokens_before.saturating_sub(tokens_after);
        let current_usage_percent = if self.context_window > 0 {
            ((tokens_after as u64 * 100) / self.context_window as u64).min(100) as u32
        } else {
            0
        };

        self.messages = new_messages;

        // Notify frontend — silent, non-intrusive
        let _ = tx.try_send(StreamEvent::ContextTrimmed {
            rounds_removed: removed_count,
            tokens_freed,
            current_usage_percent,
            context_window: self.context_window,
        });
    }

    /// Run the agent loop for a single user message.
    /// Events are streamed through `tx`. Returns total usage on completion.
    pub async fn run(
        &mut self,
        user_message: String,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<Usage, anyhow::Error> {
        self.messages.push(ChatMessage {
            role: "user".into(),
            content: Some(Value::String(user_message)),
            tool_calls: None,
            tool_call_id: None,
        });

        // ── Pre-flight: estimate tokens & trim if needed ──
        let estimated = self.estimate_total_tokens();
        let context_remaining = self.context_window.saturating_sub(estimated);
        tx.send(StreamEvent::TokenUsage {
            phase: "before".into(),
            estimated,
            actual: None,
            context_window: self.context_window,
            context_remaining,
        })
        .await
        .ok();

        self.trim_context(&tx);

        let mut total_usage = Usage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        };

        for _round in 0..MAX_TOOL_ROUNDS {
            // Build tool definitions from registry
            let tool_defs: Vec<ToolDef> = self
                .tools
                .list()
                .iter()
                .map(|t| ToolDef {
                    tool_type: "function".to_string(),
                    function: FunctionDef {
                        name: t.name().to_string(),
                        description: t.description().to_string(),
                        parameters: t.parameters(),
                    },
                })
                .collect();

            // Create intermediate channel: backend streams here, agent processes and forwards to caller
            let (backend_tx, mut backend_rx) = mpsc::channel::<StreamEvent>(256);

            let backend = self.backend.clone();
            let params = ChatParams {
                messages: self.messages.clone(),
                tools: tool_defs.clone(),
                temperature: None,
                max_tokens: None,
            };

            // Spawn the backend call so we can process events concurrently
            let chat_handle = tokio::spawn(async move { backend.chat(params, backend_tx).await });

            // Collect tool calls and usage from this round
            let mut tool_calls: Vec<(String, String, String)> = Vec::new();
            let mut round_usage = Usage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            };
            let mut had_error = false;

            while let Some(event) = backend_rx.recv().await {
                match &event {
                    StreamEvent::TextDelta { .. } | StreamEvent::ReasoningDelta { .. } | StreamEvent::ReasoningDone => {
                        tx.send(event).await.ok();
                    }
                    StreamEvent::ToolCall {
                        id,
                        name,
                        arguments,
                    } => {
                        tool_calls.push((id.clone(), name.clone(), arguments.clone()));
                        tx.send(event).await.ok();
                    }
                    StreamEvent::Done { usage } => {
                        round_usage = usage.clone();
                        // Don't forward intermediate Done — only final Done after all rounds
                    }
                    StreamEvent::Error { .. } => {
                        had_error = true;
                        tx.send(event).await.ok();
                    }
                    // Pass through new events (TokenUsage, ContextTrimmed — though ContextTrimmed
                    // is emitted by us, not the backend)
                    StreamEvent::TokenUsage { .. } | StreamEvent::ContextTrimmed { .. } => {
                        tx.send(event).await.ok();
                    }
                    _ => {
                        tx.send(event).await.ok();
                    }
                }
            }

            // Await the chat task
            let chat_result = chat_handle.await;
            match chat_result {
                Ok(Ok(_)) => {}
                Ok(Err(e)) => {
                    tx.send(StreamEvent::Error {
                        message: format!("Backend error: {}", e),
                    })
                    .await
                    .ok();
                    return Err(e);
                }
                Err(join_err) => {
                    let msg = format!("Chat task panicked: {}", join_err);
                    tx.send(StreamEvent::Error {
                        message: msg.clone(),
                    })
                    .await
                    .ok();
                    return Err(anyhow::anyhow!(msg));
                }
            }

            // Accumulate usage
            total_usage.prompt_tokens += round_usage.prompt_tokens;
            total_usage.completion_tokens += round_usage.completion_tokens;
            total_usage.total_tokens += round_usage.total_tokens;

            // ── Post-round: emit token usage with actual API data ──
            {
                let current_estimated = self.estimate_total_tokens();
                let best = current_estimated.max(total_usage.total_tokens);
                let remaining = self.context_window.saturating_sub(best);
                tx.send(StreamEvent::TokenUsage {
                    phase: "after".into(),
                    estimated: current_estimated,
                    actual: Some(total_usage.total_tokens),
                    context_window: self.context_window,
                    context_remaining: remaining,
                })
                .await
                .ok();
            }

            // If no tool calls or error, we're done
            if tool_calls.is_empty() || had_error {
                tx.send(StreamEvent::Done {
                    usage: total_usage.clone(),
                })
                .await
                .ok();
                return Ok(total_usage);
            }

            // Add assistant response placeholder (for history context)
            let assistant_tool_calls: Vec<Value> = tool_calls
                .iter()
                .map(|(id, name, args)| {
                    serde_json::json!({
                        "id": id,
                        "type": "function",
                        "function": {
                            "name": name,
                            "arguments": args,
                        }
                    })
                })
                .collect();

            self.messages.push(ChatMessage {
                role: "assistant".into(),
                content: None,
                tool_calls: Some(assistant_tool_calls),
                tool_call_id: None,
            });

            // Execute each tool call
            for (tool_id, tool_name, tool_args) in &tool_calls {
                let is_dangerous = self
                    .tools
                    .list()
                    .iter()
                    .any(|t| t.name() == tool_name && t.is_dangerous());

                // Check approval for dangerous tools
                if is_dangerous {
                    tx.send(StreamEvent::ApprovalRequired {
                        tool: tool_name.clone(),
                        params: tool_args.clone(),
                    })
                    .await
                    .ok();

                    if !self.approval_handler.approve(tool_name, tool_args).await {
                        let deny_msg =
                            format!("Tool execution denied by user: {}", tool_name);
                        self.messages.push(ChatMessage {
                            role: "tool".into(),
                            content: Some(Value::String(deny_msg.clone())),
                            tool_calls: None,
                            tool_call_id: Some(tool_id.clone()),
                        });
                        tx.send(StreamEvent::ToolResult {
                            id: tool_id.clone(),
                            content: deny_msg,
                        })
                        .await
                        .ok();
                        continue;
                    }
                }

                // Parse arguments
                let args: Value = match serde_json::from_str(tool_args) {
                    Ok(v) => v,
                    Err(e) => {
                        let err_msg =
                            format!("Failed to parse tool arguments: {}", e);
                        self.messages.push(ChatMessage {
                            role: "tool".into(),
                            content: Some(Value::String(err_msg.clone())),
                            tool_calls: None,
                            tool_call_id: Some(tool_id.clone()),
                        });
                        tx.send(StreamEvent::ToolResult {
                            id: tool_id.clone(),
                            content: err_msg,
                        })
                        .await
                        .ok();
                        continue;
                    }
                };

                // Find and execute the tool
                let result = match self.tools.get(tool_name) {
                    Some(tool) => tool.execute(args).await,
                    None => crate::tools::ToolResult {
                        success: false,
                        content: String::new(),
                        error: Some(format!("Unknown tool: {}", tool_name)),
                    },
                };

                // Emit structured events for special tools BEFORE computing result_content
                if result.success {
                    if tool_name == "todo_write" {
                        if let Ok(todos_val) = serde_json::from_str::<Value>(&result.content) {
                            if let Some(arr) = todos_val.get("todos").and_then(|v| v.as_array()) {
                                let items: Vec<crate::backends::TodoItem> = arr
                                    .iter()
                                    .filter_map(|item| {
                                        Some(crate::backends::TodoItem {
                                            content: item
                                                .get("content")?
                                                .as_str()?
                                                .to_string(),
                                            status: item
                                                .get("status")?
                                                .as_str()?
                                                .to_string(),
                                        })
                                    })
                                    .collect();
                                tx.send(StreamEvent::TodoUpdate { todos: items })
                                    .await
                                    .ok();
                            }
                        }
                    }

                    if tool_name == "propose_plan" {
                        if let Ok(plan) = serde_json::from_str::<Value>(&result.content) {
                            let title = plan
                                .get("title")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let steps: Vec<String> = plan
                                .get("steps")
                                .and_then(|v| v.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|s| s.as_str().map(String::from))
                                        .collect()
                                })
                                .unwrap_or_default();
                            let files: Vec<String> = plan
                                .get("files_to_modify")
                                .and_then(|v| v.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|s| s.as_str().map(String::from))
                                        .collect()
                                })
                                .unwrap_or_default();
                            tx.send(StreamEvent::PlanProposed {
                                title,
                                steps,
                                files_to_modify: files,
                            })
                            .await
                            .ok();
                        }
                    }
                }

                let result_content = if result.success {
                    result.content.clone()
                } else {
                    format!(
                        "Error: {}",
                        result.error.as_deref().unwrap_or("unknown error")
                    )
                };

                self.messages.push(ChatMessage {
                    role: "tool".into(),
                    content: Some(Value::String(result.content.clone())),
                    tool_calls: None,
                    tool_call_id: Some(tool_id.clone()),
                });

                tx.send(StreamEvent::ToolResult {
                    id: tool_id.clone(),
                    content: result_content,
                })
                .await
                .ok();
            }

            // ── Pre-next-round: re-estimate token usage ──
            {
                let current_estimated = self.estimate_total_tokens();
                let best = current_estimated.max(total_usage.total_tokens);
                let remaining = self.context_window.saturating_sub(best);
                tx.send(StreamEvent::TokenUsage {
                    phase: "before".into(),
                    estimated: current_estimated,
                    actual: None,
                    context_window: self.context_window,
                    context_remaining: remaining,
                })
                .await
                .ok();
            }

            // Trim before next API call if messages grew significantly
            self.trim_context(&tx);
        }

        // Hit max rounds — emit a clear, user-friendly message
        tx.send(StreamEvent::Error {
            message: format!(
                "已达到最大工具调用轮数 ({})。任务可能过于复杂，建议拆分为多个步骤逐一完成。",
                MAX_TOOL_ROUNDS
            ),
        })
        .await
        .ok();

        tx.send(StreamEvent::Done {
            usage: total_usage.clone(),
        })
        .await
        .ok();
        Ok(total_usage)
    }
}
