// MCP Client — 单个 MCP Server 的连接管理与工具调用
// 负责 initialize 握手、tools/list 发现、tools/call 调用、自动重连
// 针对 MCP Server 崩溃：检测断开 → 指数退避重连（最多 3 次）→ 标记为 Failed

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use tokio::sync::RwLock;

use crate::mcp::transport::stdio::StdioTransport;
use crate::mcp::transport::McpTransport;
use crate::mcp::types::*;

/// 最大重连次数
const MAX_RECONNECT_ATTEMPTS: u32 = 3;

/// MCP 连接状态
#[derive(Debug, Clone, PartialEq)]
pub enum McpConnectionState {
    /// 正常运行
    Connected,
    /// 连接断开，正在尝试重连
    Reconnecting,
    /// 重连失败，该 Server 工具永久不可用
    Failed(String),
}

/// MCP Server 配置（用于重连时重新 spawn 进程）
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

/// 单个 MCP Server 的客户端
/// 管理 transport 生命周期、工具缓存、连接状态
/// 调用方（McpManager）需以 Arc<McpClient> 持有，以便在断开时触发重连
pub struct McpClient {
    /// Server 名称，如 "postgres"
    pub name: String,
    /// Server 配置信息，重连时复用
    config: McpServerConfig,
    /// 当前 transport 连接
    transport: RwLock<Option<Box<dyn McpTransport>>>,
    /// 已发现的工具列表（重连后需刷新）
    pub tools: RwLock<Vec<ToolDescription>>,
    /// Server 能力声明
    pub capabilities: RwLock<ServerCapabilities>,
    /// 连接状态
    state: RwLock<McpConnectionState>,
    /// 当前连续重连次数（成功后归零）
    reconnect_count: AtomicU32,
    /// 每个实例独立的请求 ID 计数器，用于 tools/call
    call_request_id: AtomicU64,
}

impl McpClient {
    /// 创建并初始化与 MCP Server 的连接
    /// 内部执行: spawn 子进程 → initialize 握手 → tools/list 发现
    pub async fn initialize(name: String, config: McpServerConfig) -> Result<Self, McpError> {
        let client = Self {
            name,
            config,
            transport: RwLock::new(None),
            tools: RwLock::new(Vec::new()),
            capabilities: RwLock::new(ServerCapabilities { tools: None }),
            state: RwLock::new(McpConnectionState::Connected),
            reconnect_count: AtomicU32::new(0),
            call_request_id: AtomicU64::new(100),
        };

        // 首次连接
        client.do_connect().await?;
        client.do_handshake().await?;
        client.do_discover_tools().await?;

        *client.state.write().await = McpConnectionState::Connected;

        Ok(client)
    }

    // ── 内部连接方法 ──────────────────────────────────────────

    /// 创建 transport 连接（spawn 子进程）
    async fn do_connect(&self) -> Result<(), McpError> {
        let transport = StdioTransport::spawn(
            &self.config.command,
            &self.config.args,
            if self.config.env.is_empty() {
                None
            } else {
                Some(&self.config.env)
            },
        )
        .await?;

        *self.transport.write().await = Some(Box::new(transport));
        Ok(())
    }

    /// 执行 MCP initialize 握手
    async fn do_handshake(&self) -> Result<(), McpError> {
        let transport_guard = self.transport.read().await;
        let transport = transport_guard
            .as_ref()
            .ok_or_else(|| McpError::Transport("transport 未初始化".into()))?;

        // 发送 initialize 请求
        let init_params = InitializeParams::default();
        let request = JsonRpcRequest::new(
            1,
            "initialize",
            Some(
                serde_json::to_value(&init_params)
                    .map_err(|e| McpError::Protocol(e.to_string()))?,
            ),
        );

        let response = transport.send(request).await?;

        if let Some(err) = response.error {
            return Err(McpError::Protocol(format!(
                "initialize 失败: {} (code: {})",
                err.message, err.code
            )));
        }

        let init_result: InitializeResult = serde_json::from_value(
            response
                .result
                .ok_or_else(|| McpError::Protocol("initialize 响应缺少 result".into()))?,
        )
        .map_err(|e| McpError::Protocol(format!("解析 initialize 结果失败: {}", e)))?;

        *self.capabilities.write().await = init_result.capabilities;

        // 发送 initialized 通知（MCP 规范要求 handshake 完成后通知 Server）
        // 通知不需要等待响应，发送失败不阻塞流程
        drop(transport_guard);
        let _ = self
            .send_request(JsonRpcRequest::new(9999, "notifications/initialized", None))
            .await;

        Ok(())
    }

    /// 发送 tools/list 发现工具，更新本地工具缓存
    async fn do_discover_tools(&self) -> Result<(), McpError> {
        let transport_guard = self.transport.read().await;
        let transport = transport_guard
            .as_ref()
            .ok_or_else(|| McpError::Transport("transport 未初始化".into()))?;

        let request = JsonRpcRequest::new(2, "tools/list", None);
        let response = transport.send(request).await?;

        if let Some(err) = response.error {
            return Err(McpError::Protocol(format!(
                "tools/list 失败: {} (code: {})",
                err.message, err.code
            )));
        }

        let list_result: ToolsListResult = serde_json::from_value(
            response
                .result
                .ok_or_else(|| McpError::Protocol("tools/list 响应缺少 result".into()))?,
        )
        .map_err(|e| McpError::Protocol(format!("解析 tools/list 结果失败: {}", e)))?;

        *self.tools.write().await = list_result.tools;

        Ok(())
    }

    /// 通过当前 transport 发送原始请求（不处理状态）
    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
        let guard = self.transport.read().await;
        let transport = guard
            .as_ref()
            .ok_or_else(|| McpError::Transport("transport 未初始化".into()))?;
        transport.send(request).await
    }

    // ── 重连逻辑 ──────────────────────────────────────────────

    /// 自动重连：指数退避（1s / 2s / 4s），最多 3 次
    /// 重连成功后重新握手和发现工具
    /// 由 McpManager 调用（持有 Arc<McpClient>）
    pub async fn try_reconnect(&self) -> Result<(), McpError> {
        let attempts = self.reconnect_count.load(Ordering::SeqCst);

        if attempts >= MAX_RECONNECT_ATTEMPTS {
            let reason = format!(
                "MCP Server '{}' 重连失败（已尝试 {} 次），工具已不可用",
                self.name, MAX_RECONNECT_ATTEMPTS
            );
            *self.state.write().await = McpConnectionState::Failed(reason.clone());
            return Err(McpError::ConnectionFailed {
                retries: MAX_RECONNECT_ATTEMPTS,
                reason,
            });
        }

        // 先关闭旧连接
        let old = self.transport.write().await.take();
        if let Some(t) = old {
            let _ = t.close().await;
        }

        *self.state.write().await = McpConnectionState::Reconnecting;

        // 指数退避等待：1s, 2s, 4s
        let delay_secs = 1u64 << attempts;
        tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;

        self.reconnect_count.fetch_add(1, Ordering::SeqCst);

        // 重新连接、握手、发现工具
        self.do_connect().await?;
        self.do_handshake().await?;
        self.do_discover_tools().await?;

        // 成功后重置计数，标记已连接
        *self.state.write().await = McpConnectionState::Connected;
        self.reconnect_count.store(0, Ordering::SeqCst);

        Ok(())
    }

    // ── 公共方法 ──────────────────────────────────────────────

    /// 当前连接状态（异步版本）
    pub async fn state(&self) -> McpConnectionState {
        self.state.read().await.clone()
    }

    /// 同步获取连接状态快照（try_read 失败时返回 Reconnecting）
    pub fn state_sync(&self) -> McpConnectionState {
        self.state
            .try_read()
            .map(|s| s.clone())
            .unwrap_or(McpConnectionState::Reconnecting)
    }

    /// 调用 MCP Server 上的指定工具
    /// 如果调用过程中连接断开，尝试重连并重试一次
    pub async fn call_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<ToolsCallResult, McpError> {
        let current_state = self.state().await;
        match current_state {
            McpConnectionState::Failed(reason) => {
                return Err(McpError::ToolCall(format!(
                    "MCP Server '{}' 不可用: {}",
                    self.name, reason
                )));
            }
            McpConnectionState::Reconnecting => {
                return Err(McpError::ToolCall(format!(
                    "MCP Server '{}' 正在重连中，请稍后重试",
                    self.name
                )));
            }
            McpConnectionState::Connected => {}
        }

        let params = ToolsCallParams {
            name: tool_name.to_string(),
            arguments: arguments.clone(),
        };

        let request_id = self.call_request_id.fetch_add(1, Ordering::SeqCst);
        let request = JsonRpcRequest::new(
            request_id,
            "tools/call",
            Some(
                serde_json::to_value(&params)
                    .map_err(|e| McpError::Protocol(e.to_string()))?,
            ),
        );

        match self.send_request(request).await {
            Ok(response) => {
                if let Some(err) = response.error {
                    return Err(McpError::ToolCall(format!(
                        "工具执行错误: {} (code: {})",
                        err.message, err.code
                    )));
                }

                let call_result: ToolsCallResult =
                    serde_json::from_value(response.result.ok_or_else(|| {
                        McpError::Protocol("tools/call 响应缺少 result".into())
                    })?)
                    .map_err(|e| {
                        McpError::Protocol(format!("解析 tools/call 结果失败: {}", e))
                    })?;

                Ok(call_result)
            }
            Err(first_err) => {
                // 调用失败 — 尝试重连并重试一次
                // 如果重连失败，将错误透传给调用方（通常是 McpToolAdapter → LLM 可感知）
                if self.try_reconnect().await.is_ok() {
                    // 重连成功，重试调用
                    let retry_request = JsonRpcRequest::new(
                        self.call_request_id.fetch_add(1, Ordering::SeqCst),
                        "tools/call",
                        Some(
                            serde_json::to_value(&ToolsCallParams {
                                name: tool_name.to_string(),
                                arguments: arguments.clone(),
                            })
                            .map_err(|e| McpError::Protocol(e.to_string()))?,
                        ),
                    );

                    match self.send_request(retry_request).await {
                        Ok(response) => {
                            if let Some(err) = response.error {
                                return Err(McpError::ToolCall(format!(
                                    "工具执行错误: {} (code: {})",
                                    err.message, err.code
                                )));
                            }
                            let call_result: ToolsCallResult = serde_json::from_value(
                                response.result.ok_or_else(|| {
                                    McpError::Protocol("tools/call 响应缺少 result".into())
                                })?,
                            )
                            .map_err(|e| {
                                McpError::Protocol(format!("解析 tools/call 结果失败: {}", e))
                            })?;
                            return Ok(call_result);
                        }
                        Err(retry_err) => Err(retry_err),
                    }
                } else {
                    Err(first_err)
                }
            }
        }
    }

    /// 优雅关闭连接
    pub async fn shutdown(&self) -> Result<(), McpError> {
        let transport = self.transport.write().await.take();
        if let Some(t) = transport {
            t.close().await?;
        }
        *self.state.write().await = McpConnectionState::Failed("已关闭".into());
        Ok(())
    }
}
