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
    tools::{bash::BashTool, file::{ReadFileTool, WriteFileTool, ListDirTool}, grep::GrepTool, web::WebFetchTool, ToolRegistry},
};

// ---------------------------------------------------------------------------
// Approval bridge: frontend shows modal, user clicks, result returns here
// ---------------------------------------------------------------------------

/// Tauri-side ApprovalHandler. When the agent wants to run a dangerous tool,
/// this sends an event to the frontend and waits for the user's response.
struct TauriApprovalHandler {
    app_handle: tauri::AppHandle,
    /// Channel to receive the user's decision from the Tauri command.
    pending_tx: Mutex<Option<oneshot::Sender<bool>>>,
}

impl TauriApprovalHandler {
    fn new(app_handle: tauri::AppHandle) -> Self {
        Self { app_handle, pending_tx: Mutex::new(None) }
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
    let mut guard = state.pending_approval_tx.lock().await;
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
    pub pending_approval_tx: Mutex<Option<oneshot::Sender<bool>>>,
}

impl AgentSession {
    pub fn new(api_key: String, model: String, app_handle: tauri::AppHandle) -> Self {
        let backend: Arc<dyn LlmBackend> = Arc::new(DeepSeekBackend::new(model, api_key, None));

        let mut registry = ToolRegistry::new();
        registry.register(Box::new(ReadFileTool));
        registry.register(Box::new(WriteFileTool));
        registry.register(Box::new(ListDirTool));
        registry.register(Box::new(GrepTool));
        registry.register(Box::new(BashTool));
        registry.register(Box::new(WebFetchTool));

        let tools = Arc::new(registry);
        let approval: Arc<dyn ApprovalHandler> = Arc::new(TauriApprovalHandler::new(app_handle));

        let agent = AgentLoop::new(backend, tools, approval)
            .with_system_prompt(SYSTEM_PROMPT);

        Self {
            agent: Mutex::new(agent),
            cancelled: AtomicBool::new(false),
            pending_approval_tx: Mutex::new(None),
        }
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

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

#[tauri::command]
pub async fn stop_generation(
    state: tauri::State<'_, Arc<AgentSession>>,
) -> Result<(), String> {
    state.cancelled.store(true, Ordering::SeqCst);
    Ok(())
}
