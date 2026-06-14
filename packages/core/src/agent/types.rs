/// Approval handler trait - called when agent wants to execute a dangerous tool.
/// CLI implements this via stdin prompt; Desktop implements this via modal dialog.
use async_trait::async_trait;

#[async_trait]
pub trait ApprovalHandler: Send + Sync {
    /// Returns true if the tool execution is approved.
    async fn approve(&self, tool: &str, params: &str) -> bool;
}

/// Default handler that always denies (used when no handler is configured).
pub struct DenyAllHandler;

#[async_trait]
impl ApprovalHandler for DenyAllHandler {
    async fn approve(&self, _tool: &str, _params: &str) -> bool {
        false
    }
}
