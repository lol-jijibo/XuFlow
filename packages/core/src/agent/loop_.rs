/// Agent loop - the core orchestration logic.
///
/// Flow:
///   用户消息 -> 构建 messages[] -> backend.chat(stream) ->
///     ├─ text_delta -> 推送到前端流式显示
///     ├─ tool_use -> 检查危险工具 -> 发送审批事件到前端
///     ├─ tool_result -> 执行结果追加到 messages -> 继续循环
///     └─ done -> 结束本轮

use crate::agent::types::ApprovalHandler;
use crate::backends::{ChatMessage, ChatParams, FunctionDef, LlmBackend, StreamEvent, ToolDef, Usage};
use crate::tools::ToolRegistry;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;

const MAX_TOOL_ROUNDS: usize = 10;

pub struct AgentLoop {
    messages: Vec<ChatMessage>,
    backend: Arc<dyn LlmBackend>,
    tools: Arc<ToolRegistry>,
    approval_handler: Arc<dyn ApprovalHandler>,
}

impl AgentLoop {
    pub fn new(
        backend: Arc<dyn LlmBackend>,
        tools: Arc<ToolRegistry>,
        approval_handler: Arc<dyn ApprovalHandler>,
    ) -> Self {
        Self {
            messages: Vec::new(),
            backend,
            tools,
            approval_handler,
        }
    }

    /// Expose the backend for standalone operations (e.g. title summarization).
    pub fn backend(&self) -> &Arc<dyn LlmBackend> {
        &self.backend
    }

    pub fn with_system_prompt(mut self, prompt: &str) -> Self {
        self.messages.push(ChatMessage {
            role: "system".into(),
            content: Value::String(prompt.into()),
        });
        self
    }

    /// Run the agent loop for a single user message.
    /// Events are streamed through `tx`. Returns total usage on completion.
    pub async fn run(&mut self, user_message: String, tx: mpsc::Sender<StreamEvent>) -> Result<Usage, anyhow::Error> {
        self.messages.push(ChatMessage {
            role: "user".into(),
            content: Value::String(user_message),
        });

        let mut total_usage = Usage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 };

        for _round in 0..MAX_TOOL_ROUNDS {
            // Build tool definitions from registry
            let tool_defs: Vec<ToolDef> = self.tools.list().iter().map(|t| ToolDef {
                tool_type: "function".to_string(),
                function: FunctionDef {
                    name: t.name().to_string(),
                    description: t.description().to_string(),
                    parameters: t.parameters(),
                },
            }).collect();

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
            let chat_handle = tokio::spawn(async move {
                backend.chat(params, backend_tx).await
            });

            // Collect tool calls and usage from this round
            let mut tool_calls: Vec<(String, String, String)> = Vec::new();
            let mut round_usage = Usage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 };
            let mut had_error = false;

            while let Some(event) = backend_rx.recv().await {
                match &event {
                    StreamEvent::TextDelta { .. } => {
                        tx.send(event).await.ok();
                    }
                    StreamEvent::ToolCall { id, name, arguments } => {
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
                    _ => {
                        tx.send(event).await.ok();
                    }
                }
            }

            // Await the chat task
            let chat_result = chat_handle.await;
            match chat_result {
                Ok(Ok(_)) => {},
                Ok(Err(e)) => {
                    tx.send(StreamEvent::Error { message: format!("Backend error: {}", e) }).await.ok();
                    return Err(e);
                }
                Err(join_err) => {
                    let msg = format!("Chat task panicked: {}", join_err);
                    tx.send(StreamEvent::Error { message: msg.clone() }).await.ok();
                    return Err(anyhow::anyhow!(msg));
                }
            }

            // Accumulate usage
            total_usage.prompt_tokens += round_usage.prompt_tokens;
            total_usage.completion_tokens += round_usage.completion_tokens;
            total_usage.total_tokens += round_usage.total_tokens;

            // If no tool calls or error, we're done
            if tool_calls.is_empty() || had_error {
                tx.send(StreamEvent::Done { usage: total_usage.clone() }).await.ok();
                return Ok(total_usage);
            }

            // Add assistant response placeholder (for history context)
            // The actual text was streamed, but we need the tool_calls in messages
            let assistant_tool_calls: Vec<Value> = tool_calls.iter().map(|(id, name, args)| {
                serde_json::json!({
                    "id": id,
                    "type": "function",
                    "function": {
                        "name": name,
                        "arguments": args,
                    }
                })
            }).collect();

            self.messages.push(ChatMessage {
                role: "assistant".into(),
                content: Value::Object({
                    let mut map = serde_json::Map::new();
                    map.insert("tool_calls".into(), Value::Array(assistant_tool_calls));
                    map
                }),
            });

            // Execute each tool call
            for (tool_id, tool_name, tool_args) in &tool_calls {
                let is_dangerous = self.tools.list().iter().any(|t| t.name() == tool_name && t.is_dangerous());

                // Check approval for dangerous tools
                if is_dangerous {
                    tx.send(StreamEvent::ApprovalRequired {
                        tool: tool_name.clone(),
                        params: tool_args.clone(),
                    }).await.ok();

                    if !self.approval_handler.approve(tool_name, tool_args).await {
                        let deny_msg = format!("Tool execution denied by user: {}", tool_name);
                        self.messages.push(ChatMessage {
                            role: "tool".into(),
                            content: Value::String(deny_msg.clone()),
                        });
                        tx.send(StreamEvent::ToolResult {
                            id: tool_id.clone(),
                            content: deny_msg,
                        }).await.ok();
                        continue;
                    }
                }

                // Parse arguments
                let args: Value = match serde_json::from_str(tool_args) {
                    Ok(v) => v,
                    Err(e) => {
                        let err_msg = format!("Failed to parse tool arguments: {}", e);
                        self.messages.push(ChatMessage {
                            role: "tool".into(),
                            content: Value::String(err_msg.clone()),
                        });
                        tx.send(StreamEvent::ToolResult {
                            id: tool_id.clone(),
                            content: err_msg,
                        }).await.ok();
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

                let result_content = if result.success {
                    result.content.clone()
                } else {
                    format!("Error: {}", result.error.as_deref().unwrap_or("unknown error"))
                };

                self.messages.push(ChatMessage {
                    role: "tool".into(),
                    content: Value::String(result.content.clone()),
                });

                tx.send(StreamEvent::ToolResult {
                    id: tool_id.clone(),
                    content: result_content,
                }).await.ok();
            }
        }

        // Hit max rounds
        tx.send(StreamEvent::Error {
            message: format!("Reached maximum tool rounds ({})", MAX_TOOL_ROUNDS),
        }).await.ok();

        tx.send(StreamEvent::Done { usage: total_usage.clone() }).await.ok();
        Ok(total_usage)
    }
}
