// MCP 协议类型定义 — JSON-RPC 2.0 基础结构 + MCP 特定消息
// 参考 MCP 2024-11-05 规范，定义 initialize/tools/list/tools/call 的请求和响应类型

use serde::{Deserialize, Serialize};

// ── JSON-RPC 2.0 基础类型 ──────────────────────────────────────

/// JSON-RPC 2.0 请求，MCP 协议底层通信格式
/// 所有 MCP 方法调用均封装为此结构，通过 transport 发送
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcRequest {
    pub fn new(id: u64, method: &str, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            method: method.into(),
            params,
        }
    }

    /// 构造一条 JSON-RPC 通知（无 id 字段，Server 无需响应）
    pub fn notification(method: &str, params: Option<serde_json::Value>) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        map.insert("jsonrpc".into(), "2.0".into());
        map.insert("method".into(), method.into());
        if let Some(p) = params {
            map.insert("params".into(), p);
        }
        serde_json::Value::Object(map)
    }
}

/// JSON-RPC 2.0 响应，由 transport 层从子进程 stdout 解析
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<u64>,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 错误对象
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

// ── MCP initialize 握手 ────────────────────────────────────────

/// initialize 请求参数
#[derive(Debug, Serialize)]
pub struct InitializeParams {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    #[serde(rename = "clientInfo")]
    pub client_info: ClientInfo,
}

impl Default for InitializeParams {
    fn default() -> Self {
        Self {
            protocol_version: "2024-11-05".into(),
            capabilities: ClientCapabilities::default(),
            client_info: ClientInfo {
                name: "xuflow".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
        }
    }
}

/// Client 向 Server 声明的能力
#[derive(Debug, Serialize)]
pub struct ClientCapabilities {
    #[serde(default)]
    pub tools: Option<serde_json::Value>,
}

impl Default for ClientCapabilities {
    fn default() -> Self {
        Self {
            tools: Some(serde_json::json!({})),
        }
    }
}

/// Client 自身信息，Server 可用于日志或遥测
#[derive(Debug, Serialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

/// initialize 响应结果，包含 Server 的协议版本和能力声明
#[derive(Debug, Deserialize)]
pub struct InitializeResult {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
    pub capabilities: ServerCapabilities,
}

/// MCP Server 的基本信息
#[derive(Debug, Clone, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

/// Server 能力声明，Client 据此判断可以使用哪些功能
#[derive(Debug, Clone, Deserialize)]
pub struct ServerCapabilities {
    #[serde(default)]
    pub tools: Option<ToolsCapability>,
}

/// Server 是否支持工具列表变更通知
#[derive(Debug, Clone, Deserialize)]
pub struct ToolsCapability {
    #[serde(rename = "listChanged", default)]
    pub list_changed: Option<bool>,
}

// ── MCP 工具发现与调用 ────────────────────────────────────────

/// 单个 MCP 工具的描述，来自 tools/list 响应
/// input_schema 为 JSON Schema 格式，可直接映射到 OpenAI FunctionDef.parameters
#[derive(Debug, Clone, Deserialize)]
pub struct ToolDescription {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

/// tools/list 的 result 字段
#[derive(Debug, Clone, Deserialize)]
pub struct ToolsListResult {
    pub tools: Vec<ToolDescription>,
}

/// tools/call 的 params 字段
#[derive(Debug, Clone, Serialize)]
pub struct ToolsCallParams {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// tools/call 的 result 字段
#[derive(Debug, Clone, Deserialize)]
pub struct ToolsCallResult {
    pub content: Vec<ToolContent>,
    #[serde(rename = "isError", default)]
    pub is_error: Option<bool>,
}

/// 工具返回内容条目，支持 text/image/resource 三种类型
#[derive(Debug, Clone, Deserialize)]
pub struct ToolContent {
    #[serde(rename = "type")]
    pub content_type: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub data: Option<String>,
    #[serde(default)]
    pub mime_type: Option<String>,
}

// ── 错误类型 ────────────────────────────────────────────────────

/// MCP 协议层的所有错误类型
/// 覆盖 transport、协议解析、工具调用三类错误，集成到 McpClient 和 McpToolAdapter 中
#[derive(Debug, Clone, thiserror::Error)]
pub enum McpError {
    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Tool call error: {0}")]
    ToolCall(String),

    #[error("Connection failed after {retries} retries: {reason}")]
    ConnectionFailed { retries: u32, reason: String },

    #[error("Timeout ({0}s)")]
    Timeout(u64),
}
