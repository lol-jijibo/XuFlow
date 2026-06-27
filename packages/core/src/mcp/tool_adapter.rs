// MCP 工具适配器 — 将远程 MCP 工具包装为本地 Tool trait 实现
// 每个 MCP 工具对应一个 McpToolAdapter 实例，注册到 ToolRegistry 中
// AgentLoop 通过统一的 Tool trait 调用，不感知工具是内置还是 MCP

use async_trait::async_trait;
use std::sync::Arc;

use crate::mcp::client::{McpClient, McpConnectionState};
use crate::tools::{Tool, ToolResult};

/// MCP 工具包装器，将远程 MCP Server 的工具暴露为本地 Tool trait
pub struct McpToolAdapter {
    /// 完整工具名，格式 "mcp__{server}__{tool}"，避免与内置工具冲突
    full_name: String,
    /// 原始工具名（MCP Server 侧的名称，不含 mcp__ 前缀）
    tool_name: String,
    /// Server 名称，用于错误消息
    server_name: String,
    /// 工具描述，带 [MCP: server_name] 标记，让 LLM 知道这是外部工具
    description: String,
    /// JSON Schema 参数定义，原样来自 tools/list
    parameters: serde_json::Value,
    /// 是否标记为危险（MCP 工具默认 true，在配置中可豁免为 false）
    is_dangerous: bool,
    /// 指向所属 MCP Server 连接，用于发送 tools/call
    client: Arc<McpClient>,
}

impl McpToolAdapter {
    pub fn new(
        server_name: String,
        tool_desc: &crate::mcp::types::ToolDescription,
        client: Arc<McpClient>,
        is_dangerous: bool,
    ) -> Self {
        let full_name = format!("mcp__{}__{}", server_name, tool_desc.name);
        let description = format!("[MCP:{}] {}", server_name, tool_desc.description);

        Self {
            full_name,
            tool_name: tool_desc.name.clone(),
            server_name,
            description,
            parameters: tool_desc.input_schema.clone(),
            is_dangerous,
            client,
        }
    }
}

#[async_trait]
impl Tool for McpToolAdapter {
    fn name(&self) -> &str {
        &self.full_name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters(&self) -> serde_json::Value {
        self.parameters.clone()
    }

    fn is_dangerous(&self) -> bool {
        self.is_dangerous
    }

    async fn execute(&self, arguments: serde_json::Value) -> ToolResult {
        // 检查连接状态，避免在已断开时无谓发送请求
        match self.client.state().await {
            McpConnectionState::Failed(reason) => {
                return ToolResult {
                    success: false,
                    content: String::new(),
                    error: Some(format!(
                        "MCP Server '{}' 不可用: {}",
                        self.server_name, reason
                    )),
                };
            }
            McpConnectionState::Reconnecting => {
                return ToolResult {
                    success: false,
                    content: String::new(),
                    error: Some(format!(
                        "MCP Server '{}' 正在重连，请稍后重试",
                        self.server_name
                    )),
                };
            }
            McpConnectionState::Connected => {}
        }

        match self.client.call_tool(&self.tool_name, arguments).await {
            Ok(call_result) => {
                let is_error = call_result.is_error.unwrap_or(false);
                // 提取所有 text 类型的内容片段，拼接为结果字符串
                let content: String = call_result
                    .content
                    .iter()
                    .filter(|c| c.content_type == "text")
                    .filter_map(|c| c.text.as_deref())
                    .collect::<Vec<_>>()
                    .join("\n");

                ToolResult {
                    success: !is_error,
                    content,
                    error: if is_error {
                        Some("MCP Server 返回错误".into())
                    } else {
                        None
                    },
                }
            }
            Err(e) => ToolResult {
                success: false,
                content: String::new(),
                error: Some(format!(
                    "MCP 工具调用失败 [{}::{}]: {}",
                    self.server_name, self.tool_name, e
                )),
            },
        }
    }
}
