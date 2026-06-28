// MCP（Model Context Protocol）模块入口
// 管理所有 MCP Server 连接的生命周期：加载配置 → 连接 → 发现工具 → 注册 → 关闭
// 连接失败不阻塞启动，收集错误供前端展示

pub mod client;
pub mod config;
pub mod tool_adapter;
pub mod transport;
pub mod types;

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::mcp::client::McpClient;
use crate::mcp::config::{load_mcp_config, load_mcp_config_multi, resolve_global_config_path};
use crate::mcp::tool_adapter::McpToolAdapter;
use crate::tools::ToolRegistry;

/// 单个 MCP Server 初始化失败的错误信息
#[derive(Debug, Clone)]
pub struct McpInitError {
    pub server_name: String,
    pub message: String,
}

/// MCP 连接管理器 — 持有所有 MCP Server 的客户端实例
/// 负责：加载配置 → 批量连接 → 工具注册 → 优雅关闭
pub struct McpManager {
    clients: Vec<Arc<McpClient>>,
    /// 每个 Server 的安全工具名列表（is_dangerous = false）
    safe_tools_map: HashMap<String, Vec<String>>,
}

impl McpManager {
    /// 从配置文件加载并连接所有 MCP Server
    /// 当 global_config_path 为 None 时自动解析：环境变量 > 默认路径
    /// 连接失败的 Server 不阻塞整体流程，错误信息通过第二个返回值收集
    pub async fn load_from_config(
        global_config_path: Option<&Path>,
        project_root: Option<&Path>,
    ) -> (Self, Vec<McpInitError>) {
        // 未显式指定路径时，使用智能解析（环境变量 > 默认路径）
        let resolved = global_config_path.map(|p| p.to_path_buf()).or_else(resolve_global_config_path);
        let (config, warnings) = load_mcp_config(resolved.as_deref(), project_root);
        Self::connect_all(config, warnings).await
    }

    /// 使用智能路径解析加载 MCP 配置
    /// 路径优先级：explicit_path > 环境变量 XUFLOW_MCP_CONFIG > 默认路径 + 项目级
    pub async fn load_with_resolution(
        explicit_path: Option<&Path>,
        project_root: Option<&Path>,
    ) -> (Self, Vec<McpInitError>) {
        let (config, warnings) = load_mcp_config_multi(explicit_path, project_root);
        Self::connect_all(config, warnings).await
    }

    /// 根据已合并的配置并发连接所有 MCP Server
    async fn connect_all(
        config: crate::mcp::config::MergedMcpConfig,
        warnings: Vec<String>,
    ) -> (Self, Vec<McpInitError>) {
        let mut clients: Vec<Arc<McpClient>> = Vec::new();
        let mut errors: Vec<McpInitError> = Vec::new();

        for w in warnings {
            errors.push(McpInitError {
                server_name: String::new(),
                message: w,
            });
        }

        for (name, server_config) in config.servers {
            match McpClient::initialize(name.clone(), server_config).await {
                Ok(client) => {
                    clients.push(Arc::new(client));
                }
                Err(e) => {
                    errors.push(McpInitError {
                        server_name: name,
                        message: format!("MCP Server 初始化失败: {}", e),
                    });
                }
            }
        }

        let safe_tools_map = config.safe_tools;
        (Self { clients, safe_tools_map }, errors)
    }

    /// 将所有已连接 Server 的工具注入 ToolRegistry
    /// 返回注入的工具总数
    pub fn register_tools(&self, registry: &mut ToolRegistry) -> usize {
        let mut count = 0;

        for client in &self.clients {
            // 使用 try_read 避免异步等待 — 此时所有 client 已初始化完毕
            let tools = match client.tools.try_read() {
                Ok(guard) => guard.clone(),
                Err(_) => continue,
            };

            let server_name = &client.name;
            let safe_tools = self.safe_tools_map.get(server_name);

            for tool_desc in &tools {
                // 用 safeTools 配置决定工具是否为危险工具
                // 工具默认 is_dangerous=true，仅在 safeTools 列表中时豁免
                let is_dangerous = match safe_tools {
                    Some(list) => !list.contains(&tool_desc.name),
                    None => true,
                };

                let adapter = McpToolAdapter::new(
                    server_name.clone(),
                    tool_desc,
                    Arc::clone(client),
                    is_dangerous,
                );

                registry.register(Box::new(adapter));
                count += 1;
            }
        }

        count
    }

    /// 根据 Server 名获取 MCP 连接状态摘要，供前端展示（同步版本）
    pub fn status_summary(&self) -> Vec<McpServerStatus> {
        self.clients
            .iter()
            .map(|c| {
                let state = c.state_sync();
                McpServerStatus {
                    name: c.name.clone(),
                    tool_count: c.tools.try_read().map(|t| t.len()).unwrap_or(0),
                    connected: matches!(state, client::McpConnectionState::Connected),
                }
            })
            .collect()
    }

    /// 关闭所有 MCP Server 连接
    pub async fn shutdown_all(&self) {
        for client in &self.clients {
            let _ = client.shutdown().await;
        }
    }
}

/// MCP Server 状态摘要，用于前端状态展示
#[derive(Debug, Clone)]
pub struct McpServerStatus {
    pub name: String,
    pub tool_count: usize,
    pub connected: bool,
}
