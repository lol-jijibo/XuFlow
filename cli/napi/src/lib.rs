// napi-rs bindings for xuflow-core
// Exposes Agent, Backends, Tools, Config, and SessionStore to Node.js

use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ErrorStrategy, ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use std::sync::Arc;
use xuflow_core::agent::loop_::AgentLoop;
use xuflow_core::agent::types::{ApprovalHandler, DenyAllHandler};
use xuflow_core::backends::{
    deepseek::DeepSeekBackend,
    kimi::KimiBackend,
    openai_compat::OpenAICompatBackend,
    volcengine::VolcEngineBackend,
    LlmBackend, StreamEvent,
};
use xuflow_core::config::{AppConfig, ModelConfig};
use xuflow_core::memory::session::SessionStore;
use xuflow_core::tools::{
    bash::BashTool,
    file::{ListDirTool, ReadFileTool, WriteFileTool},
    grep::GrepTool,
    web::WebFetchTool,
    ToolRegistry,
};

// ─── Backend wrappers ────────────────────────────────────────

#[napi]
pub struct DeepSeek {
    inner: Arc<dyn LlmBackend>,
}

#[napi]
impl DeepSeek {
    #[napi(constructor)]
    pub fn new(model: String, api_key: String, base_url: Option<String>) -> Self {
        Self {
            inner: Arc::new(DeepSeekBackend::new(model, api_key, base_url)),
        }
    }
}

#[napi]
pub struct VolcEngine {
    inner: Arc<dyn LlmBackend>,
}

#[napi]
impl VolcEngine {
    #[napi(constructor)]
    pub fn new(model: String, api_key: String, base_url: Option<String>) -> Self {
        Self {
            inner: Arc::new(VolcEngineBackend::new(model, api_key, base_url)),
        }
    }
}

#[napi]
pub struct Kimi {
    inner: Arc<dyn LlmBackend>,
}

#[napi]
impl Kimi {
    #[napi(constructor)]
    pub fn new(model: String, api_key: String, base_url: Option<String>) -> Self {
        Self {
            inner: Arc::new(KimiBackend::new(model, api_key, base_url)),
        }
    }
}

#[napi]
pub struct OpenAICompat {
    inner: Arc<dyn LlmBackend>,
}

#[napi]
impl OpenAICompat {
    #[napi(constructor)]
    pub fn new(model: String, api_key: String, base_url: String) -> Self {
        Self {
            inner: Arc::new(OpenAICompatBackend::new(model, api_key, base_url)),
        }
    }
}

// ─── Tool Registry ───────────────────────────────────────────

#[napi]
pub struct JsToolRegistry {
    inner: Arc<ToolRegistry>,
}

#[napi]
impl JsToolRegistry {
    #[napi(constructor)]
    pub fn new() -> Self {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(ReadFileTool));
        registry.register(Box::new(WriteFileTool));
        registry.register(Box::new(ListDirTool));
        registry.register(Box::new(GrepTool));
        registry.register(Box::new(BashTool));
        registry.register(Box::new(WebFetchTool));
        Self {
            inner: Arc::new(registry),
        }
    }
}

// ─── Approval Handler (JS callback) ──────────────────────────

/// JS-implemented approval handler that calls back into Node.js.
struct JsApprovalHandler {
    callback: ThreadsafeFunction<(String, String), ErrorStrategy::Fatal>,
}

impl JsApprovalHandler {
    fn new(callback: ThreadsafeFunction<(String, String), ErrorStrategy::Fatal>) -> Self {
        Self { callback }
    }
}

#[async_trait::async_trait]
impl ApprovalHandler for JsApprovalHandler {
    async fn approve(&self, tool: &str, params: &str) -> bool {
        let tool = tool.to_string();
        let params = params.to_string();
        let callback = self.callback.clone();

        match callback.call_async((tool, params)).await {
            Ok(result) => result,
            Err(_) => false,
        }
    }
}

// ─── Agent Loop ──────────────────────────────────────────────

type EventCallback = ThreadsafeFunction<String, ErrorStrategy::Fatal>;

#[napi]
pub struct JsAgentLoop {
    agent: tokio::sync::Mutex<AgentLoop>,
    callback: std::sync::Mutex<Option<EventCallback>>,
}

#[napi]
impl JsAgentLoop {
    #[napi(constructor)]
    pub fn new(
        backend: &JsBackend,
        tools: &JsToolRegistry,
        system_prompt: Option<String>,
    ) -> Self {
        let approval_handler: Arc<dyn ApprovalHandler> = Arc::new(DenyAllHandler);
        let mut agent = AgentLoop::new(
            backend.inner.clone(),
            tools.inner.clone(),
            approval_handler,
        );
        if let Some(prompt) = system_prompt {
            agent = agent.with_system_prompt(&prompt);
        }
        Self {
            agent: tokio::sync::Mutex::new(agent),
            callback: std::sync::Mutex::new(None),
        }
    }

    /// Set the event callback (JS function) before calling run().
    /// This is a sync method because JsFunction is not Send-safe in async context.
    #[napi]
    pub fn set_event_callback(&self, callback: JsFunction) -> napi::Result<()> {
        let tsfn: EventCallback = callback.create_threadsafe_function(0, |ctx| {
            Ok(vec![ctx.value])
        })?;
        *self.callback.lock().unwrap() = Some(tsfn);
        Ok(())
    }

    /// Run the agent loop with a user message.
    /// Events are streamed via the callback set by set_event_callback.
    /// Returns JSON string of the final Usage.
    #[napi]
    pub async fn run(&self, user_message: String) -> napi::Result<String> {
        let tsfn = {
            let lock = self.callback.lock().unwrap();
            lock.as_ref()
                .ok_or_else(|| {
                    napi::Error::from_reason(
                        "Event callback not set. Call set_event_callback() before run().",
                    )
                })?
                .clone()
        };

        let (tx, mut rx) = tokio::sync::mpsc::channel::<StreamEvent>(256);

        let mut agent = self.agent.lock().await;

        // Spawn a task that forwards events from the channel to JS via ThreadsafeFunction
        let handle = {
            let tsfn = tsfn.clone();
            tokio::spawn(async move {
                while let Some(event) = rx.recv().await {
                    let json = serde_json::to_string(&event).unwrap_or_default();
                    tsfn.call(json, ThreadsafeFunctionCallMode::NonBlocking);
                }
            })
        };

        match agent.run(user_message, tx).await {
            Ok(usage) => {
                // Give the forwarder a moment to flush the Done event
                handle.await.ok();
                Ok(serde_json::to_string(&usage).unwrap_or_default())
            }
            Err(e) => {
                handle.await.ok();
                Err(napi::Error::from_reason(format!("Agent error: {}", e)))
            }
        }
    }
}

// ─── Backend enum for AgentLoop constructor ──────────────────

#[napi]
pub struct JsBackend {
    inner: Arc<dyn LlmBackend>,
}

#[napi]
impl JsBackend {
    #[napi(factory)]
    pub fn from_deepseek(backend: &DeepSeek) -> Self {
        Self {
            inner: backend.inner.clone(),
        }
    }

    #[napi(factory)]
    pub fn from_volcengine(backend: &VolcEngine) -> Self {
        Self {
            inner: backend.inner.clone(),
        }
    }

    #[napi(factory)]
    pub fn from_kimi(backend: &Kimi) -> Self {
        Self {
            inner: backend.inner.clone(),
        }
    }

    #[napi(factory)]
    pub fn from_openai_compat(backend: &OpenAICompat) -> Self {
        Self {
            inner: backend.inner.clone(),
        }
    }
}

// ─── Session Store ───────────────────────────────────────────

#[napi]
pub struct JsSessionStore {
    store: std::sync::Mutex<SessionStore>,
}

#[napi]
impl JsSessionStore {
    #[napi(constructor)]
    pub fn new(db_path: Option<String>) -> napi::Result<Self> {
        let path = db_path.map(std::path::PathBuf::from);
        let store = SessionStore::open(path)
            .map_err(|e| napi::Error::from_reason(format!("Failed to open database: {}", e)))?;
        Ok(Self {
            store: std::sync::Mutex::new(store),
        })
    }

    #[napi]
    pub fn create_session(&self, id: String, title: String) -> napi::Result<String> {
        let session = self
            .store
            .lock()
            .unwrap()
            .create_session(&id, &title)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(serde_json::to_string(&session).unwrap_or_default())
    }

    #[napi]
    pub fn list_sessions(&self) -> napi::Result<String> {
        let sessions = self
            .store
            .lock()
            .unwrap()
            .list_sessions()
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(serde_json::to_string(&sessions).unwrap_or_default())
    }

    #[napi]
    pub fn get_session(&self, id: String) -> napi::Result<Option<String>> {
        let session = self
            .store
            .lock()
            .unwrap()
            .get_session(&id)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(session.map(|s| serde_json::to_string(&s).unwrap_or_default()))
    }

    #[napi]
    pub fn delete_session(&self, id: String) -> napi::Result<()> {
        self.store
            .lock()
            .unwrap()
            .delete_session(&id)
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    #[napi]
    pub fn update_session_title(&self, id: String, title: String) -> napi::Result<()> {
        self.store
            .lock()
            .unwrap()
            .update_session_title(&id, &title)
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    #[napi]
    pub fn add_message(
        &self,
        session_id: String,
        role: String,
        content: String,
    ) -> napi::Result<String> {
        let msg = self
            .store
            .lock()
            .unwrap()
            .add_message(&session_id, &role, &content)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(serde_json::to_string(&msg).unwrap_or_default())
    }

    #[napi]
    pub fn get_messages(&self, session_id: String) -> napi::Result<String> {
        let messages = self
            .store
            .lock()
            .unwrap()
            .get_messages(&session_id)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(serde_json::to_string(&messages).unwrap_or_default())
    }

    #[napi]
    pub fn clear_messages(&self, session_id: String) -> napi::Result<()> {
        self.store
            .lock()
            .unwrap()
            .clear_messages(&session_id)
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }
}

// ─── Config helpers ──────────────────────────────────────────

#[napi]
pub fn create_default_config() -> String {
    let config = AppConfig::default();
    serde_json::to_string(&config).unwrap_or_default()
}

#[napi]
pub fn parse_model_config(json: String) -> napi::Result<String> {
    let config: ModelConfig = serde_json::from_str(&json)
        .map_err(|e| napi::Error::from_reason(format!("Invalid config JSON: {}", e)))?;
    Ok(serde_json::to_string(&config).unwrap_or_default())
}

#[napi]
pub fn get_system_prompt(working_dir: String) -> String {
    xuflow_core::agent::system_prompt::build_system_prompt(&working_dir)
}
