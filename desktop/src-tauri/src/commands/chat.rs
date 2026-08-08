use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Emitter;
use tokio::sync::{mpsc, Mutex, oneshot};

use xuflow_core::{
    agent::loop_::AgentLoop,
    agent::system_prompt::build_system_prompt,
    agent::types::ApprovalHandler,
    backends::{ChatMessage, ChatParams, LlmBackend, StreamEvent},
    backends::deepseek::DeepSeekBackend,
    backends::kimi::KimiBackend,
    backends::volcengine::VolcEngineBackend,
    mcp::McpManager,
    tools::{bash::BashTool, edit::EditFileTool, file::{ReadFileTool, WriteFileTool, ListDirTool}, git::{GitStatusTool, GitDiffTool, GitLogTool, GitAddTool, GitCommitTool}, glob::GlobTool, grep::GrepTool, patch::ApplyPatchTool, todo::{TodoWriteTool, ProposePlanTool}, web::WebFetchTool, web_crawl::WebCrawlTool, web_search::WebSearchTool, ToolRegistry},
};

// ---------------------------------------------------------------------------
// Shared approval channel — single source of truth for both the
// TauriApprovalHandler (writer) and respond_approval command (reader).
// ---------------------------------------------------------------------------

type ApprovalChannel = Arc<Mutex<Option<oneshot::Sender<bool>>>>;

// ---------------------------------------------------------------------------
// Approval bridge: frontend shows modal, user clicks, result returns here
// ---------------------------------------------------------------------------

/// Tauri-side ApprovalHandler. When the agent wants to run a dangerous tool,
/// this sends an event to the frontend and waits for the user's response.
struct TauriApprovalHandler {
    app_handle: tauri::AppHandle,
    /// Shared channel — the handler writes, respond_approval reads.
    pending_tx: ApprovalChannel,
}

impl TauriApprovalHandler {
    fn new(app_handle: tauri::AppHandle, pending_tx: ApprovalChannel) -> Self {
        Self { app_handle, pending_tx }
    }
}

#[async_trait::async_trait]
impl ApprovalHandler for TauriApprovalHandler {
    async fn approve(&self, tool: &str, params: &str) -> bool {
        let (tx, rx) = oneshot::channel::<bool>();
        *self.pending_tx.lock().await = Some(tx);

        let payload = serde_json::json!({ "tool": tool, "params": params });
        let _ = self.app_handle.emit("agent:approval-required", payload.to_string());

        // Wait for the user to respond (or timeout)
        match tokio::time::timeout(std::time::Duration::from_secs(180), rx).await {
            Ok(Ok(approved)) => approved,
            _ => false, // timeout or channel closed → deny
        }
    }
}

// ---------------------------------------------------------------------------
// Per-session handle — each conversation gets its own AgentLoop
// ---------------------------------------------------------------------------

/// 单个会话的运行时状态：独立的消息历史、取消标志、审批通道。
/// 不同会话可并行运行，互不阻塞。
struct SessionHandle {
    agent: Mutex<AgentLoop>,
    cancelled: Arc<AtomicBool>,
    approval_tx: ApprovalChannel,
}

// ---------------------------------------------------------------------------
// Global config — updated by configure_agent, read when creating new sessions
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct GlobalConfig {
    api_key: String,
    model: String,
    provider: String,
}

// ---------------------------------------------------------------------------
// Session manager — replaces the old single AgentSession
// ---------------------------------------------------------------------------

/// 会话管理器：按 sessionId 维护多个 AgentLoop 实例的池。
/// 每个会话独立持有消息历史、取消标志和审批通道，支持并行运行。
/// MCP 管理器和应用配置全局共享。
pub struct SessionManager {
    /// 活跃会话池：sessionId → Handle
    sessions: Mutex<HashMap<String, Arc<SessionHandle>>>,
    /// 全局凭证和模型选择（configure_agent 更新，新建会话时消费）
    global_config: Mutex<GlobalConfig>,
    /// 全局 MCP 管理器（所有会话共享，首次 configure_agent 时初始化）
    mcp_manager: Arc<Mutex<Option<Arc<McpManager>>>>,
    /// MCP 初始化警告/错误信息
    mcp_init_errors: Arc<Mutex<Vec<String>>>,
    /// AppHandle 克隆（事件发射和 approval handler 使用）
    app_handle: tauri::AppHandle,
    /// 工作目录
    working_dir: String,
}

impl SessionManager {
    /// 创建新的会话管理器。前端必须在首次 send_message 前调用 configure_agent 设置凭证。
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        let working_dir = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string());

        Self {
            sessions: Mutex::new(HashMap::new()),
            global_config: Mutex::new(GlobalConfig {
                api_key: String::new(),
                model: "deepseek-v4-pro".into(),
                provider: "deepseek".into(),
            }),
            mcp_manager: Arc::new(Mutex::new(None)),
            mcp_init_errors: Arc::new(Mutex::new(Vec::new())),
            app_handle,
            working_dir,
        }
    }

    // ── Backend / Agent 构造（静态方法，不依赖 self）──────────────────

    fn build_backend(provider: &str, model: &str, api_key: &str) -> Arc<dyn LlmBackend> {
        match provider {
            "volcengine" => Arc::new(VolcEngineBackend::new(model.to_string(), api_key.to_string(), None)),
            "kimi" => Arc::new(KimiBackend::new(model.to_string(), api_key.to_string(), None)),
            _ => Arc::new(DeepSeekBackend::new(model.to_string(), api_key.to_string(), None)),
        }
    }

    fn build_agent(
        backend: Arc<dyn LlmBackend>,
        app_handle: tauri::AppHandle,
        approval_tx: ApprovalChannel,
        working_dir: &str,
        mcp_manager: Option<Arc<McpManager>>,
        cancelled: Arc<AtomicBool>,
    ) -> AgentLoop {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(ReadFileTool));
        registry.register(Box::new(WriteFileTool));
        registry.register(Box::new(EditFileTool));
        registry.register(Box::new(ApplyPatchTool));
        registry.register(Box::new(ListDirTool));
        registry.register(Box::new(GrepTool));
        registry.register(Box::new(BashTool));
        registry.register(Box::new(WebFetchTool));
        registry.register(Box::new(WebSearchTool));
        registry.register(Box::new(WebCrawlTool));
        registry.register(Box::new(GlobTool));
        registry.register(Box::new(GitStatusTool));
        registry.register(Box::new(GitDiffTool));
        registry.register(Box::new(GitLogTool));
        registry.register(Box::new(GitAddTool));
        registry.register(Box::new(GitCommitTool));
        registry.register(Box::new(TodoWriteTool));
        registry.register(Box::new(ProposePlanTool));

        // 将 MCP Server 提供的工具注册到 ToolRegistry
        // 注册后的工具与内置工具在同一列表中，AgentLoop 统一调用
        if let Some(ref mcp) = mcp_manager {
            mcp.register_tools(&mut registry);
        }

        let tools = Arc::new(registry);
        let approval: Arc<dyn ApprovalHandler> = Arc::new(TauriApprovalHandler::new(app_handle, approval_tx));

        let system_prompt = build_system_prompt(working_dir);
        AgentLoop::new(backend, tools, approval)
            .with_system_prompt(&system_prompt)
            .with_cancellation(cancelled)
    }

    // ── Session lifecycle ──────────────────────────────────────────────

    /// 获取或创建指定 sessionId 的会话 Handle。
    /// 如果会话已存在则直接返回，否则从全局配置构造新的 AgentLoop。
    /// messages 用于恢复已有会话的消息历史（空 Vec 表示新会话）。
    async fn get_or_create_session(
        &self,
        session_id: &str,
        messages: Vec<ChatMessage>,
    ) -> Arc<SessionHandle> {
        let mut sessions = self.sessions.lock().await;

        if let Some(handle) = sessions.get(session_id) {
            return handle.clone();
        }

        // 构造新会话
        let cfg = self.global_config.lock().await.clone();
        let mcp = self.mcp_manager.lock().await.clone();
        let backend = Self::build_backend(&cfg.provider, &cfg.model, &cfg.api_key);
        let cancelled = Arc::new(AtomicBool::new(false));
        let approval_tx: ApprovalChannel = Arc::new(Mutex::new(None));

        let agent = if messages.is_empty() {
            Self::build_agent(
                backend,
                self.app_handle.clone(),
                approval_tx.clone(),
                &self.working_dir,
                mcp,
                cancelled.clone(),
            )
        } else {
            Self::build_agent(
                backend,
                self.app_handle.clone(),
                approval_tx.clone(),
                &self.working_dir,
                mcp,
                cancelled.clone(),
            )
            .with_messages(messages)
        };

        let handle = Arc::new(SessionHandle {
            agent: Mutex::new(agent),
            cancelled,
            approval_tx,
        });

        sessions.insert(session_id.to_string(), handle.clone());
        handle
    }

    /// 清理指定会话，释放内存和 AgentLoop 资源。
    async fn remove_session(&self, session_id: &str) {
        let mut sessions = self.sessions.lock().await;
        // 先设置取消标志，让正在运行的 AgentLoop 感知并退出
        if let Some(handle) = sessions.get(session_id) {
            handle.cancelled.store(true, Ordering::SeqCst);
        }
        sessions.remove(session_id);
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// 推送凭证和模型选择到后端，并在首次调用时初始化 MCP 连接。
/// 必须在 send_message 之前至少调用一次；用户修改设置后也应调用。
#[tauri::command]
pub async fn configure_agent(
    api_key: String,
    provider: String,
    model: String,
    state: tauri::State<'_, Arc<SessionManager>>,
    app: tauri::AppHandle,
    mcp_config_path: Option<String>,
) -> Result<(), String> {
    // 更新全局配置
    {
        let mut cfg = state.global_config.lock().await;
        cfg.api_key = api_key;
        cfg.model = model;
        cfg.provider = provider;
    }

    // 首次调用时延迟初始化 MCP，后续不重复加载
    let mcp_needs_init = {
        let guard = state.mcp_manager.lock().await;
        guard.is_none()
    };

    if mcp_needs_init {
        let (manager, init_errors) = McpManager::load_with_resolution(
            mcp_config_path.as_deref().map(std::path::Path::new),
            Some(std::path::Path::new(&state.working_dir)),
        )
        .await;

        // 收集初始化警告
        let mut errors_guard = state.mcp_init_errors.lock().await;
        errors_guard.clear();
        for err in &init_errors {
            if err.server_name.is_empty() {
                errors_guard.push(err.message.clone());
            } else {
                errors_guard.push(format!(
                    "MCP Server '{}': {}",
                    err.server_name, err.message
                ));
            }
        }

        *state.mcp_manager.lock().await = Some(Arc::new(manager));
    }

    // 让 MCP init errors 可用（后续可添加查询命令）
    let _ = app;

    Ok(())
}

/// 从 SQLite 恢复会话的消息历史并创建/替换后端 AgentLoop。
/// 前端在切换到已有会话时调用，确保后端持有完整上下文。
#[tauri::command]
pub async fn restore_session(
    session_id: String,
    messages_json: String,
    state: tauri::State<'_, Arc<SessionManager>>,
) -> Result<(), String> {
    // 解析前端传来的消息列表
    let raw_msgs: Vec<serde_json::Value> =
        serde_json::from_str(&messages_json).map_err(|e| format!("Invalid messages JSON: {}", e))?;

    // 转换为 ChatMessage 格式
    // 恢复时保留 role + content，tool_call 相关字段暂不还原（简化处理）
    let chat_messages: Vec<ChatMessage> = raw_msgs
        .iter()
        .map(|m| ChatMessage {
            role: m.get("role").and_then(|v| v.as_str()).unwrap_or("user").to_string(),
            content: m.get("content").cloned(),
            tool_calls: None,
            tool_call_id: None,
        })
        .collect();

    // 移除旧会话（如果存在），用新消息历史重建
    state.remove_session(&session_id).await;

    // 通过 get_or_create 重建（带消息历史）
    state.get_or_create_session(&session_id, chat_messages).await;

    Ok(())
}

/// 关闭并清理指定会话的后端资源。
/// 前端在切换走一个会话后延迟调用（5s），避免频繁切换导致反复重建。
#[tauri::command]
pub async fn close_session(
    session_id: String,
    state: tauri::State<'_, Arc<SessionManager>>,
) -> Result<(), String> {
    state.remove_session(&session_id).await;
    Ok(())
}

#[tauri::command]
pub async fn send_message(
    session_id: String,
    content: String,
    state: tauri::State<'_, Arc<SessionManager>>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    // 获取或创建会话（新会话无历史消息）
    let handle = state.get_or_create_session(&session_id, vec![]).await;

    // 重置取消标志
    handle.cancelled.store(false, Ordering::SeqCst);

    let (tx, mut rx) = mpsc::channel::<StreamEvent>(256);

    // 事件转发器 —— 所有 payload 附带 session_id 以便前端过滤
    let cancelled = handle.cancelled.clone();
    let app_clone = app.clone();
    let sid = session_id.clone();
    let forward_handle = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            if cancelled.load(Ordering::SeqCst) {
                break;
            }
            match &event {
                StreamEvent::TextDelta { delta } => {
                    let payload = serde_json::json!({ "session_id": sid, "delta": delta });
                    let _ = app_clone.emit("agent:text-delta", payload.to_string());
                }
                StreamEvent::ReasoningDelta { delta } => {
                    let payload = serde_json::json!({ "session_id": sid, "delta": delta });
                    let _ = app_clone.emit("agent:reasoning-delta", payload.to_string());
                }
                StreamEvent::ReasoningDone => {
                    let payload = serde_json::json!({ "session_id": sid });
                    let _ = app_clone.emit("agent:reasoning-done", payload.to_string());
                }
                StreamEvent::ToolCall { id, name, arguments } => {
                    let payload = serde_json::json!({
                        "session_id": sid,
                        "id": id, "name": name, "arguments": arguments
                    });
                    let _ = app_clone.emit("agent:tool-call", payload.to_string());
                }
                StreamEvent::ToolResult { id, content } => {
                    let payload = serde_json::json!({
                        "session_id": sid,
                        "id": id, "content": content
                    });
                    let _ = app_clone.emit("agent:tool-result", payload.to_string());
                }
                StreamEvent::ApprovalRequired { tool, params } => {
                    let payload = serde_json::json!({
                        "session_id": sid,
                        "tool": tool, "params": params
                    });
                    let _ = app_clone.emit("agent:approval-required", payload.to_string());
                }
                StreamEvent::TokenUsage { phase, estimated, actual, context_window, context_remaining } => {
                    let payload = serde_json::json!({
                        "session_id": sid,
                        "phase": phase,
                        "estimated": estimated,
                        "actual": actual,
                        "context_window": context_window,
                        "context_remaining": context_remaining,
                    });
                    let _ = app_clone.emit("agent:token-usage", payload.to_string());
                }
                StreamEvent::ContextTrimmed { rounds_removed, tokens_freed, current_usage_percent, context_window } => {
                    let payload = serde_json::json!({
                        "session_id": sid,
                        "rounds_removed": rounds_removed,
                        "tokens_freed": tokens_freed,
                        "current_usage_percent": current_usage_percent,
                        "context_window": context_window,
                    });
                    let _ = app_clone.emit("agent:context-trimmed", payload.to_string());
                }
                StreamEvent::ContextSummarized { turns_summarized, summary_length } => {
                    let payload = serde_json::json!({
                        "session_id": sid,
                        "turns_summarized": turns_summarized,
                        "summary_length": summary_length,
                    });
                    let _ = app_clone.emit("agent:context-summarized", payload.to_string());
                }
                StreamEvent::Done { usage } => {
                    let payload = serde_json::json!({
                        "session_id": sid,
                        "v": 1,
                        "usage": {
                            "prompt_tokens": usage.prompt_tokens,
                            "completion_tokens": usage.completion_tokens,
                            "total_tokens": usage.total_tokens,
                        }
                    });
                    let _ = app_clone.emit("agent:done", payload.to_string());
                }
                StreamEvent::Error { message } => {
                    let payload = serde_json::json!({
                        "session_id": sid,
                        "message": message
                    });
                    let _ = app_clone.emit("agent:error", payload.to_string());
                }
                StreamEvent::TodoUpdate { todos } => {
                    let payload = serde_json::json!({
                        "session_id": sid,
                        "todos": todos
                    });
                    let _ = app_clone.emit("agent:todo-update", payload.to_string());
                }
                StreamEvent::PlanProposed { title, steps, files_to_modify } => {
                    let payload = serde_json::json!({
                        "session_id": sid,
                        "title": title,
                        "steps": steps,
                        "files_to_modify": files_to_modify,
                    });
                    let _ = app_clone.emit("agent:plan-proposed", payload.to_string());
                }
            }
        }
    });

    // 运行 AgentLoop
    let agent_result = {
        let mut agent_guard = handle.agent.lock().await;
        agent_guard.run(content, tx).await
    };

    let _ = forward_handle.await;

    match agent_result {
        Ok(usage) => Ok(format!("{} tokens", usage.total_tokens)),
        Err(e) => Err(format!("Agent error: {}", e)),
    }
}

/// Read API keys from system environment variables.
/// Looks for DEEP_SEEK_API_KEY, ARK_API_KEY (Volcengine/Ark), and KIMI_API_KEY.
#[tauri::command]
pub fn get_env_api_keys() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "deepseek_api_key": std::env::var("DEEP_SEEK_API_KEY").unwrap_or_default(),
        "ark_api_key": std::env::var("ARK_API_KEY").unwrap_or_default(),
        "kimi_api_key": std::env::var("KIMI_API_KEY").unwrap_or_default(),
    }))
}

/// 停止指定会话的生成。session_id 为空时停止所有会话。
#[tauri::command]
pub async fn stop_generation(
    session_id: Option<String>,
    state: tauri::State<'_, Arc<SessionManager>>,
) -> Result<(), String> {
    if let Some(sid) = session_id {
        // 取消指定会话
        let sessions = state.sessions.lock().await;
        if let Some(handle) = sessions.get(&sid) {
            handle.cancelled.store(true, Ordering::SeqCst);
        }
    } else {
        // 向后兼容：无 sessionId 时取消所有会话
        let sessions = state.sessions.lock().await;
        for handle in sessions.values() {
            handle.cancelled.store(true, Ordering::SeqCst);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Context management commands
// ---------------------------------------------------------------------------

/// Set the context window size for the specified session's agent.
#[tauri::command]
pub async fn set_context_window(
    session_id: String,
    context_window: u32,
    state: tauri::State<'_, Arc<SessionManager>>,
) -> Result<(), String> {
    let sessions = state.sessions.lock().await;
    if let Some(handle) = sessions.get(&session_id) {
        let mut agent = handle.agent.lock().await;
        agent.set_context_window(context_window);
    }
    Ok(())
}

/// Set the minimum user turns to preserve during context trimming.
#[tauri::command]
pub async fn set_min_user_turns(
    session_id: String,
    min_turns: u32,
    state: tauri::State<'_, Arc<SessionManager>>,
) -> Result<(), String> {
    let sessions = state.sessions.lock().await;
    if let Some(handle) = sessions.get(&session_id) {
        let mut agent = handle.agent.lock().await;
        agent.set_min_user_turns(min_turns);
    }
    Ok(())
}

/// 响应审批请求（指定会话）。
#[tauri::command]
pub async fn respond_approval(
    session_id: String,
    approved: bool,
    state: tauri::State<'_, Arc<SessionManager>>,
) -> Result<(), String> {
    let sessions = state.sessions.lock().await;
    if let Some(handle) = sessions.get(&session_id) {
        let mut guard = handle.approval_tx.lock().await;
        if let Some(tx) = guard.take() {
            let _ = tx.send(approved);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Title summarization — non-streaming, no-tool chat for conversation titles
// ---------------------------------------------------------------------------

/// Run a simple non-streaming completion through the backend, collecting all
/// text deltas into a single result string.
async fn simple_completion(
    backend: &Arc<dyn LlmBackend>,
    messages: Vec<ChatMessage>,
) -> Result<String, String> {
    let (tx, mut rx) = mpsc::channel::<StreamEvent>(256);
    let params = ChatParams {
        messages,
        tools: vec![],
        temperature: Some(0.3),
        max_tokens: Some(60),
    };

    let b = backend.clone();
    tokio::spawn(async move { b.chat(params, tx).await });

    let mut text = String::new();
    while let Some(event) = rx.recv().await {
        match event {
            StreamEvent::TextDelta { delta } => text.push_str(&delta),
            StreamEvent::Done { .. } => break,
            StreamEvent::Error { message } => {
                return Err(format!("Summarization failed: {}", message));
            }
            _ => {} // ignore tool-call etc. (shouldn't happen without tools)
        }
    }
    Ok(text.trim().to_string())
}

/// Build a summarization prompt from the conversation messages (JSON).
/// Returns (system_prompt, user_prompt).
fn build_summary_prompt(messages_json: &str) -> Result<(String, String), String> {
    #[derive(serde::Deserialize)]
    struct Msg {
        role: String,
        content: String,
    }

    let msgs: Vec<Msg> =
        serde_json::from_str(messages_json).map_err(|e| format!("Invalid messages JSON: {}", e))?;

    let user_msgs: Vec<&Msg> = msgs.iter().filter(|m| m.role == "user").collect();

    if user_msgs.is_empty() {
        return Err("No user messages found".into());
    }

    let system_prompt = "You are a title generator. Generate a concise, descriptive title (max 30 characters) for the conversation. Return ONLY the title — no quotes, no explanations, no prefixes.".to_string();

    if user_msgs.len() == 1 {
        let content = user_msgs[0].content.trim();
        let user_prompt = format!(
            "Generate a short title (max 30 chars) for a conversation that starts with this query:\n\n{content}\n\nTitle:"
        );
        Ok((system_prompt, user_prompt))
    } else {
        // Multi-turn: include the full conversation excerpt
        let mut conv_text = String::new();
        for msg in &msgs {
            let role_label = match msg.role.as_str() {
                "user" => "User",
                "assistant" => "Assistant",
                _ => continue,
            };
            // Truncate long messages in the prompt to save tokens
            let excerpt: String = if msg.content.len() > 300 {
                format!("{}...", &msg.content[..300])
            } else {
                msg.content.clone()
            };
            conv_text.push_str(&format!("{role_label}: {excerpt}\n"));
        }

        let user_prompt = format!(
            "Generate a short title (max 30 chars) that captures the main topic of this conversation:\n\n{conv_text}\nTitle:"
        );
        Ok((system_prompt, user_prompt))
    }
}

/// Generate a conversation title by summarizing the message history.
/// Called by the frontend after each complete agent response.
/// 使用全局配置构造临时 backend，不依赖特定会话的 AgentLoop。
#[tauri::command]
pub async fn generate_title(
    messages_json: String,
    state: tauri::State<'_, Arc<SessionManager>>,
) -> Result<String, String> {
    let (system_prompt, user_prompt) = build_summary_prompt(&messages_json)?;

    let messages = vec![
        ChatMessage {
            role: "system".into(),
            content: Some(serde_json::Value::String(system_prompt)),
            tool_calls: None,
            tool_call_id: None,
        },
        ChatMessage {
            role: "user".into(),
            content: Some(serde_json::Value::String(user_prompt)),
            tool_calls: None,
            tool_call_id: None,
        },
    ];

    // 使用全局配置构造临时后端，生成标题
    let cfg = state.global_config.lock().await.clone();
    let backend = SessionManager::build_backend(&cfg.provider, &cfg.model, &cfg.api_key);

    simple_completion(&backend, messages).await
}
