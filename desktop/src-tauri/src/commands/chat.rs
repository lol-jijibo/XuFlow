use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Emitter;
use tokio::sync::{mpsc, Mutex, oneshot};

use xuflow_core::{
    agent::loop_::AgentLoop,
    agent::system_prompt::SYSTEM_PROMPT,
    agent::types::ApprovalHandler,
    backends::{LlmBackend, StreamEvent},
    backends::deepseek::DeepSeekBackend,
    backends::volcengine::VolcEngineBackend,
    tools::{bash::BashTool, file::{ReadFileTool, WriteFileTool, ListDirTool}, grep::GrepTool, web::WebFetchTool, ToolRegistry},
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

/// Tauri command: called by the frontend when the user approves or rejects.
#[tauri::command]
pub async fn respond_approval(
    approved: bool,
    state: tauri::State<'_, Arc<AgentSession>>,
) -> Result<(), String> {
    let mut guard = state.approval_tx.lock().await;
    if let Some(tx) = guard.take() {
        let _ = tx.send(approved);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Agent session
// ---------------------------------------------------------------------------

pub struct AgentSession {
    pub agent: Mutex<AgentLoop>,
    pub cancelled: AtomicBool,
    /// Shared approval channel — also held by TauriApprovalHandler.
    pub approval_tx: ApprovalChannel,
}

impl AgentSession {
    /// Create a new session. Call `configure_agent` from the frontend to set
    /// real credentials before the first `send_message`.
    pub fn new(api_key: String, model: String, provider: String, app_handle: tauri::AppHandle) -> Self {
        let approval_tx: ApprovalChannel = Arc::new(Mutex::new(None));

        let backend = Self::build_backend(&provider, &model, &api_key);
        let agent = Self::build_agent(backend, app_handle.clone(), approval_tx.clone());

        Self {
            agent: Mutex::new(agent),
            cancelled: AtomicBool::new(false),
            approval_tx,
        }
    }

    /// Rebuild the backend and agent loop with new credentials / model.
    pub async fn reconfigure(&self, api_key: String, model: String, provider: String, app_handle: tauri::AppHandle) {
        let backend = Self::build_backend(&provider, &model, &api_key);
        let agent = Self::build_agent(backend, app_handle, self.approval_tx.clone());
        *self.agent.lock().await = agent;
    }

    fn build_backend(provider: &str, model: &str, api_key: &str) -> Arc<dyn LlmBackend> {
        match provider {
            "volcengine" => Arc::new(VolcEngineBackend::new(model.to_string(), api_key.to_string(), None)),
            _ => Arc::new(DeepSeekBackend::new(model.to_string(), api_key.to_string(), None)),
        }
    }

    fn build_agent(
        backend: Arc<dyn LlmBackend>,
        app_handle: tauri::AppHandle,
        approval_tx: ApprovalChannel,
    ) -> AgentLoop {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(ReadFileTool));
        registry.register(Box::new(WriteFileTool));
        registry.register(Box::new(ListDirTool));
        registry.register(Box::new(GrepTool));
        registry.register(Box::new(BashTool));
        registry.register(Box::new(WebFetchTool));

        let tools = Arc::new(registry);
        let approval: Arc<dyn ApprovalHandler> = Arc::new(TauriApprovalHandler::new(app_handle, approval_tx));

        AgentLoop::new(backend, tools, approval)
            .with_system_prompt(SYSTEM_PROMPT)
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Called by the frontend to push credentials & model selection to the backend.
/// Must be invoked at least once before `send_message`, and again whenever the
/// user changes provider / model / API key.
#[tauri::command]
pub async fn configure_agent(
    api_key: String,
    provider: String,
    model: String,
    state: tauri::State<'_, Arc<AgentSession>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    state.reconfigure(api_key, model, provider, app).await;
    Ok(())
}

#[tauri::command]
pub async fn send_message(
    content: String,
    state: tauri::State<'_, Arc<AgentSession>>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    // Reset cancellation
    state.cancelled.store(false, Ordering::SeqCst);

    let (tx, mut rx) = mpsc::channel::<StreamEvent>(256);

    // Spawn event forwarder — clone the Arc so we can read cancelled
    let session = state.inner().clone();
    let app_clone = app.clone();
    let forward_handle = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            if session.cancelled.load(Ordering::SeqCst) {
                break;
            }
            match &event {
                StreamEvent::TextDelta { delta } => {
                    let _ = app_clone.emit("agent:text-delta", delta);
                }
                StreamEvent::ToolCall { id, name, arguments } => {
                    let payload = serde_json::json!({
                        "id": id, "name": name, "arguments": arguments
                    });
                    let _ = app_clone.emit("agent:tool-call", payload.to_string());
                }
                StreamEvent::ToolResult { id, content } => {
                    let payload = serde_json::json!({
                        "id": id, "content": content
                    });
                    let _ = app_clone.emit("agent:tool-result", payload.to_string());
                }
                StreamEvent::ApprovalRequired { tool, params } => {
                    let payload = serde_json::json!({
                        "tool": tool, "params": params
                    });
                    let _ = app_clone.emit("agent:approval-required", payload.to_string());
                }
                StreamEvent::Done { .. } => {
                    let _ = app_clone.emit("agent:done", ());
                }
                StreamEvent::Error { message } => {
                    let _ = app_clone.emit("agent:error", message);
                }
            }
        }
    });

    // Run the agent
    let agent_result = {
        let mut agent_guard = state.agent.lock().await;
        agent_guard.run(content, tx).await
    };

    let _ = forward_handle.await;

    match agent_result {
        Ok(usage) => Ok(format!("{} tokens", usage.total_tokens)),
        Err(e) => Err(format!("Agent error: {}", e)),
    }
}

/// Read API keys from system environment variables.
/// Looks for DEEP_SEEK_API_KEY and ARK_API_KEY (Volcengine/Ark).
#[tauri::command]
pub fn get_env_api_keys() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "deepseek_api_key": std::env::var("DEEP_SEEK_API_KEY").unwrap_or_default(),
        "ark_api_key": std::env::var("ARK_API_KEY").unwrap_or_default(),
    }))
}

#[tauri::command]
pub async fn stop_generation(
    state: tauri::State<'_, Arc<AgentSession>>,
) -> Result<(), String> {
    state.cancelled.store(true, Ordering::SeqCst);
    Ok(())
}
