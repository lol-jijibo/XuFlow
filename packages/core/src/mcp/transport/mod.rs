// MCP Transport 抽象层
// 定义统一的 Transport trait，支持 stdio（本地子进程）和未来扩展 SSE
pub mod stdio;

use async_trait::async_trait;
use crate::mcp::types::{JsonRpcRequest, JsonRpcResponse, McpError};

/// MCP Transport 统一接口
/// 抽象底层通信方式，让 McpClient 不感知是本地进程还是远端服务
#[async_trait]
pub trait McpTransport: Send + Sync {
    /// 发送 JSON-RPC 请求，阻塞等待并返回响应
    async fn send(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, McpError>;

    /// 连接是否仍存活（用于检测子进程退出 / 网络断开）
    fn is_alive(&self) -> bool;

    /// 关闭连接并释放资源（kill 子进程 / 关闭 HTTP 连接）
    async fn close(&self) -> Result<(), McpError>;
}
