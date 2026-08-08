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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
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
    /// 取消标志：由 Tauri stop_generation 命令写入，Agent 循环每轮检查。
    /// 为 None 时忽略取消逻辑（非 Tauri 场景兼容）。
    cancelled: Option<Arc<AtomicBool>>,
    /// 累积的早期对话摘要（LLM 生成，替代丢弃的旧 turn）。
    summary: String,
    /// 是否启用摘要压缩。默认 true；可通过 set_summarization_enabled 关闭。
    enable_summarization: bool,
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
            cancelled: None,
            summary: String::new(),
            enable_summarization: true,
        }
    }

    /// 注入取消令牌：Agent 循环在每轮 API 调用前后检查此标志，
    /// 一旦被外部设为 true 即立即中止执行并返回。
    pub fn with_cancellation(mut self, cancelled: Arc<AtomicBool>) -> Self {
        self.cancelled = Some(cancelled);
        self
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

    /// 批量注入已有消息历史（从 SQLite 恢复会话上下文时使用）。
    /// 直接替换当前 messages，后续 run() 在此基础上追加新消息。
    pub fn with_messages(mut self, messages: Vec<ChatMessage>) -> Self {
        self.messages = messages;
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

    /// Enable or disable LLM summarization of old conversation turns.
    /// When disabled, old turns are simply dropped (legacy behavior).
    pub fn set_summarization_enabled(&mut self, enabled: bool) {
        self.enable_summarization = enabled;
    }

    /// Whether summarization is currently enabled.
    pub fn summarization_enabled(&self) -> bool {
        self.enable_summarization
    }

    // ── Token estimation ──────────────────────────────────────────────

    /// Estimate total tokens for all messages currently in context.
    fn estimate_total_tokens(&self) -> u32 {
        token_counter::estimate_total_tokens(&self.messages, &self.token_config)
    }

    // ── Dynamic context trimming (with optional LLM summarization) ────

    /// 对即将被丢弃的对话轮次生成 LLM 摘要，保留关键决策、代码变更和用户意图。
    /// 失败时返回 None，调用方降级为直接丢弃。
    async fn summarize_turns(&self, turns: &[Vec<ChatMessage>]) -> Option<String> {
        if turns.is_empty() {
            return None;
        }

        // 构建待摘要的对话文本
        let mut transcript = String::new();
        for turn in turns {
            for msg in turn {
                let role_label = match msg.role.as_str() {
                    "user" => "用户",
                    "assistant" => "助手",
                    "tool" => continue, // 工具结果通常很长且没有摘要价值
                    _ => continue,
                };
                if let Some(content) = &msg.content {
                    let text = match content {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    // 截断过长的单条消息
                    let excerpt: String = if text.len() > 500 {
                        format!("{}...", &text[..500])
                    } else {
                        text
                    };
                    transcript.push_str(&format!("{role_label}: {excerpt}\n"));
                }
            }
        }

        if transcript.trim().is_empty() {
            return None;
        }

        // 构建摘要 prompt
        let summary_prompt = format!(
            "请用中文将以下对话片段压缩为一段简洁的摘要（最多 200 字），\
             保留关键决策、代码变更内容和用户的核心意图：\n\n{transcript}\n\n摘要："
        );

        // 如果有旧摘要，要求模型合并
        let user_content = if self.summary.is_empty() {
            summary_prompt
        } else {
            format!(
                "已有的历史摘要：\n{}\n\n请将以下新对话与已有摘要合并为一段摘要（最多 300 字）：\n\n{transcript}\n\n合并后的摘要：",
                self.summary
            )
        };

        let messages = vec![
            ChatMessage {
                role: "system".into(),
                content: Some(Value::String(
                    "你是一个对话摘要助手。输出纯文本摘要，不含前缀、标签或格式标记。".into(),
                )),
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: "user".into(),
                content: Some(Value::String(user_content)),
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let params = ChatParams {
            messages,
            tools: vec![],
            temperature: Some(0.3),
            max_tokens: Some(300),
        };

        // 通过临时通道获取摘要结果（与 simple_completion 模式一致）
        let (sum_tx, mut sum_rx) = mpsc::channel::<StreamEvent>(32);
        let backend = self.backend.clone();

        let chat_task = tokio::spawn(async move { backend.chat(params, sum_tx).await });

        let mut summary_text = String::new();
        // 10 秒超时，防止摘要调用阻塞主流程
        let timeout = tokio::time::timeout(Duration::from_secs(10), async {
            while let Some(event) = sum_rx.recv().await {
                match event {
                    StreamEvent::TextDelta { delta } => summary_text.push_str(&delta),
                    StreamEvent::Done { .. } => break,
                    StreamEvent::Error { .. } => return Err(()),
                    _ => {}
                }
            }
            Ok(())
        })
        .await;

        match timeout {
            Ok(Ok(_)) => {
                let text = summary_text.trim().to_string();
                if text.is_empty() {
                    None
                } else {
                    Some(text)
                }
            }
            _ => {
                // 超时或后端错误 → 降级，返回 None 让调用方直接丢弃
                let _ = chat_task.abort();
                None
            }
        }
    }

    /// Trim older conversation turns to stay within the context window.
    ///
    /// Algorithm:
    ///   1. If estimated tokens < 80% of context_window, do nothing.
    ///   2. Group messages into atomic "turns" (user msg + all following
    ///      assistant/tool msgs until the next user msg).
    ///   3. From the end, mark the last `min_user_turns` turns as protected.
    ///   4. If summarization is enabled, generate an LLM summary of the turns
    ///      about to be dropped and inject it as context before discarding.
    ///   5. Drop oldest unprotected turns one by one, re-estimating after
    ///      each drop, until usage < 60% or only protected turns remain.
    ///   6. Emit ContextTrimmed and ContextSummarized events.
    async fn trim_context(&mut self, tx: &mpsc::Sender<StreamEvent>) {
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

        // ── 摘要压缩：先对即将被丢弃的 turn 做 LLM 摘要 ──
        if self.enable_summarization && protected_start > 0 {
            let to_summarize: Vec<Vec<ChatMessage>> =
                turns[..protected_start].to_vec();

            if let Some(new_summary) = self.summarize_turns(&to_summarize).await {
                // 更新累积摘要
                self.summary = new_summary;

                let _ = tx.try_send(StreamEvent::ContextSummarized {
                    turns_summarized: protected_start as u32,
                    summary_length: self.summary.len() as u32,
                });

                // 将摘要注入为一条特殊的 user 消息，放在受保护的 turn 之前。
                // 使用 user 角色而非 system，确保不同 API 都能正确传递。
                let summary_msg = ChatMessage {
                    role: "user".into(),
                    content: Some(Value::String(format!(
                        "[历史对话摘要]\n{}",
                        self.summary
                    ))),
                    tool_calls: None,
                    tool_call_id: None,
                };

                // 重建 messages：system + summary + 受保护的 turn
                let mut new_msgs = system_msgs.clone();
                new_msgs.push(summary_msg);
                for turn in &turns[protected_start..] {
                    new_msgs.extend(turn.iter().cloned());
                }

                // 检查是否仍需进一步裁剪
                let current_tokens = token_counter::estimate_total_tokens(&new_msgs, &self.token_config);
                let target = self.context_window.saturating_mul(60) / 100;
                if current_tokens < target {
                    self.messages = new_msgs;
                    let current_usage_percent = if self.context_window > 0 {
                        ((current_tokens as u64 * 100) / self.context_window as u64).min(100) as u32
                    } else {
                        0
                    };
                    let _ = tx.try_send(StreamEvent::ContextTrimmed {
                        rounds_removed: protected_start as u32,
                        tokens_freed: estimated.saturating_sub(current_tokens),
                        current_usage_percent,
                        context_window: self.context_window,
                    });
                    return;
                }

                // 摘要后仍超阈值 → 使用新 message 状态继续裁剪
                self.messages = new_msgs;
                return; // 一次摘要足以释放空间；如果仍不够，下次循环再裁剪
            }
            // 摘要失败 → 降级为原始丢弃逻辑（继续执行下方代码）
        }

        // ── 丢弃式裁剪（无摘要或摘要失败时的降级路径）──
        let tokens_before = token_counter::estimate_total_tokens(
            &turns.iter().flatten().cloned().collect::<Vec<_>>(),
            &self.token_config,
        );

        let mut removed_count: u32 = 0;

        let target = self.context_window.saturating_mul(60) / 100;

        while turns.len() > self.min_user_turns as usize {
            turns.remove(0);
            removed_count += 1;

            let current_tokens = token_counter::estimate_total_tokens(
                &turns.iter().flatten().cloned().collect::<Vec<_>>(),
                &self.token_config,
            );
            if current_tokens < target {
                break;
            }
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

        self.trim_context(&tx).await;

        let mut total_usage = Usage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        };

        for _round in 0..MAX_TOOL_ROUNDS {
            // ── 检查取消标志：用户点击停止后立即中止本轮 ──
            if let Some(ref cancelled) = self.cancelled {
                if cancelled.load(Ordering::SeqCst) {
                    let _ = tx.send(StreamEvent::Done { usage: total_usage.clone() }).await;
                    return Ok(total_usage);
                }
            }
            // Build tool definitions from registry
            // 针对 MCP 工具数量过多导致 prompt 膨胀的问题，采用多级截断：
            //   ≤ 20 个 MCP 工具：全量发送完整定义
            //   21-40 个 MCP 工具：MCP 工具只保留 name + description，JSON Schema 置空
            //   > 40 个 MCP 工具：按 Server 聚合为摘要行，LLM 需要细节时通过内置工具查询
            let tool_defs: Vec<ToolDef> = build_tool_defs_with_truncation(self.tools.list());

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

            // 在接收后端事件的同时轮询取消标志（每 200ms 检查一次），
            // 确保用户点击停止后流式输出能在 200ms 内中止，不再等待 LLM 响应完成。
            let mut chat_done = false;
            while !chat_done {
                // 先检查取消标志，避免在已取消状态下继续轮询
                if let Some(ref cancelled) = self.cancelled {
                    if cancelled.load(Ordering::SeqCst) {
                        chat_handle.abort();
                        let _ = tx.send(StreamEvent::Done { usage: total_usage.clone() }).await;
                        return Ok(total_usage);
                    }
                }

                match tokio::time::timeout(Duration::from_millis(200), backend_rx.recv()).await {
                    Ok(Some(event)) => {
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
                    Ok(None) => {
                        chat_done = true;
                    }
                    Err(_) => {
                        // 超时 —— 回到循环顶部再次检查取消标志
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

            // ── 工具执行后再次检查取消标志，避免进入下一轮 API 调用 ──
            if let Some(ref cancelled) = self.cancelled {
                if cancelled.load(Ordering::SeqCst) {
                    let _ = tx.send(StreamEvent::Done { usage: total_usage.clone() }).await;
                    return Ok(total_usage);
                }
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
            self.trim_context(&tx).await;
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

/// 构建工具定义列表，对 MCP 工具实施多级截断以防 prompt 膨胀
/// MCP 工具通过名称前缀 "mcp__" 识别
/// 截断策略:
///   - MCP 工具 ≤ 20: 完整定义（含 JSON Schema）
///   - MCP 工具 21-40: 只保留 name + description，parameters 设为 {{}}
///   - MCP 工具 > 40: 按 Server 聚合为摘要，每个 Server 一行
/// 内置工具始终全量发送
fn build_tool_defs_with_truncation(tools: &[Box<dyn crate::tools::Tool>]) -> Vec<ToolDef> {
    let (builtins, mcps): (Vec<_>, Vec<_>) = tools
        .iter()
        .partition(|t| !t.name().starts_with("mcp__"));

    let mcp_count = mcps.len();

    let mut defs: Vec<ToolDef> = builtins
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

    match mcp_count {
        0 => {} // 无 MCP 工具
        1..=20 => {
            // 全量：每个 MCP 工具完整定义
            for t in &mcps {
                defs.push(ToolDef {
                    tool_type: "function".to_string(),
                    function: FunctionDef {
                        name: t.name().to_string(),
                        description: t.description().to_string(),
                        parameters: t.parameters(),
                    },
                });
            }
        }
        21..=40 => {
            // 降级：MCP 工具只发 name + 一行描述，parameters 为空对象
            // LLM 可凭描述判断是否调用，参数由 MCP Server 在调用时校验
            for t in &mcps {
                defs.push(ToolDef {
                    tool_type: "function".to_string(),
                    function: FunctionDef {
                        name: t.name().to_string(),
                        description: t.description().to_string(),
                        parameters: serde_json::json!({}),
                    },
                });
            }
        }
        _ => {
            // 严重降级：按 Server 聚合 MCP 工具为摘要描述
            // 聚合为: "mcp__<server>__<tool1>, <tool2>, ..."
            let mut server_map: std::collections::HashMap<String, Vec<String>> =
                std::collections::HashMap::new();
            for t in &mcps {
                let name = t.name();
                // mcp__<server>__<tool>
                let parts: Vec<&str> = name.splitn(3, "__").collect();
                if parts.len() >= 2 {
                    server_map
                        .entry(parts[1].to_string())
                        .or_default()
                        .push(parts.get(2).map(|s| s.to_string()).unwrap_or_default());
                }
            }

            for (server, tool_list) in &server_map {
                let summary = format!(
                    "MCP Server '{}' 提供的工具: {}",
                    server,
                    tool_list.join(", ")
                );
                defs.push(ToolDef {
                    tool_type: "function".to_string(),
                    function: FunctionDef {
                        name: format!("mcp__{}__list_tools", server),
                        description: summary,
                        parameters: serde_json::json!({
                            "type": "object",
                            "properties": {},
                        }),
                    },
                });
            }
        }
    }

    defs
}

// ── 测试 ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::{Tool, ToolRegistry, ToolResult};

    /// 纯文本工具供测试使用
    struct DummyTool {
        name: String,
    }

    #[async_trait::async_trait]
    impl Tool for DummyTool {
        fn name(&self) -> &str { &self.name }
        fn description(&self) -> &str { "A dummy tool" }
        fn parameters(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "arg": { "type": "string" }
                }
            })
        }
        fn is_dangerous(&self) -> bool { false }
        async fn execute(&self, _args: Value) -> ToolResult {
            ToolResult { success: true, content: "ok".into(), error: None }
        }
    }

    fn make_mcp_tools(count: usize, server: &str) -> Vec<Box<dyn Tool>> {
        (0..count)
            .map(|i| {
                Box::new(DummyTool {
                    name: format!("mcp__{}__tool_{}", server, i),
                }) as Box<dyn Tool>
            })
            .collect()
    }

    #[test]
    fn test_tool_truncation_below_20_mcps_full_definition() {
        let builtins: Vec<Box<dyn Tool>> = vec![Box::new(DummyTool { name: "read_file".into() })];
        let mcps = make_mcp_tools(5, "filesystem");
        let all: Vec<Box<dyn Tool>> = builtins.into_iter().chain(mcps).collect();
        let defs = build_tool_defs_with_truncation(&all);
        // 1 builtin + 5 MCP = 6 full definitions
        assert_eq!(defs.len(), 6);
        for d in &defs {
            assert!(
                d.function.parameters != serde_json::json!({}),
                "工具 {} 的 parameters 不应为空", d.function.name
            );
        }
    }

    #[test]
    fn test_tool_truncation_21_to_40_mcps_parameters_empty() {
        let builtins: Vec<Box<dyn Tool>> = vec![Box::new(DummyTool { name: "read_file".into() })];
        let mcps = make_mcp_tools(25, "filesystem");
        let all: Vec<Box<dyn Tool>> = builtins.into_iter().chain(mcps).collect();
        let defs = build_tool_defs_with_truncation(&all);
        // 1 builtin (full) + 25 MCP (empty params)
        assert_eq!(defs.len(), 26);
        // 内置工具仍应有完整参数
        assert_ne!(defs[0].function.parameters, serde_json::json!({}));
        // MCP 工具的参数应为空
        assert_eq!(defs[1].function.parameters, serde_json::json!({}));
    }

    #[test]
    fn test_tool_truncation_over_40_mcps_aggregated() {
        let builtins: Vec<Box<dyn Tool>> = vec![Box::new(DummyTool { name: "read_file".into() })];
        let mcps_a = make_mcp_tools(25, "server_a");
        let mcps_b = make_mcp_tools(20, "server_b");
        let all: Vec<Box<dyn Tool>> = builtins
            .into_iter()
            .chain(mcps_a)
            .chain(mcps_b)
            .collect();
        let defs = build_tool_defs_with_truncation(&all);
        // 1 builtin + 2 aggregated server entries
        assert_eq!(defs.len(), 3);
        assert!(defs[1].function.name.contains("list_tools"));
        assert!(defs[2].function.name.contains("list_tools"));
    }

    #[test]
    fn test_agent_context_window_defaults() {
        // 纯单元测试：验证默认值初始化
        let tools = Arc::new(ToolRegistry::new());
        struct NoopApproval;
        #[async_trait::async_trait]
        impl crate::agent::types::ApprovalHandler for NoopApproval {
            async fn approve(&self, _tool: &str, _params: &str) -> bool { true }
        }

        let backend = Arc::new(crate::backends::openai_compat::OpenAICompatBackend::new(
            "test-model".into(),
            "https://example.com".into(),
            "sk-test".into(),
        ));

        let agent = AgentLoop::new(backend, tools, Arc::new(NoopApproval));
        assert_eq!(agent.context_window(), 128_000);
        assert_eq!(agent.min_user_turns(), 3);
        assert!(agent.summarization_enabled());
    }
}
