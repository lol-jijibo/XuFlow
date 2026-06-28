// MCP 配置加载 — 读取和合并 mcp_servers.json 配置
// 配置来源优先级（后者覆盖前者）：
//   1. 全局配置文件（路径由 XUFLOW_MCP_CONFIG 环境变量指定，未设置则用默认路径）
//   2. 项目级 xuflow.json 中的 mcpServers 字段
//   3. 显式传入的自定义路径（如命令行参数 / 前端设置）
// 读取失败不阻塞启动，只收集警告

use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::mcp::client::McpServerConfig;

/// 环境变量名：自定义 MCP 全局配置文件路径
const ENV_MCP_CONFIG_PATH: &str = "XUFLOW_MCP_CONFIG";

/// 顶层配置文件结构
#[derive(Debug, Deserialize, Default)]
pub struct McpServersFile {
    #[serde(rename = "mcpServers", default)]
    pub mcp_servers: HashMap<String, McpServerEntry>,
}

/// 单个 MCP Server 的配置项
#[derive(Debug, Deserialize, Clone)]
pub struct McpServerEntry {
    /// 可执行文件路径或命令名（如 "npx", "node", "python"）
    pub command: String,
    /// 命令行参数
    #[serde(default)]
    pub args: Vec<String>,
    /// 环境变量
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// 是否禁用该 Server
    #[serde(default)]
    pub disabled: bool,
    /// 标记为非危险的工具名列表（这些工具将 is_dangerous = false）
    #[serde(rename = "safeTools", default)]
    pub safe_tools: Vec<String>,
}

/// 合并后的运行时配置
#[derive(Debug, Clone)]
pub struct MergedMcpConfig {
    pub servers: HashMap<String, McpServerConfig>,
    /// 每个 Server 对应的安全工具名集合
    pub safe_tools: HashMap<String, Vec<String>>,
}

/// 加载并合并全局和项目级 MCP 配置
/// 查找顺序：全局配置 → 项目级覆盖（后者覆盖前者同名 Server）
/// 返回合并后的配置，以及加载过程中产生的警告
pub fn load_mcp_config(
    global_config_path: Option<&Path>,
    project_root: Option<&Path>,
) -> (MergedMcpConfig, Vec<String>) {
    let mut merged = MergedMcpConfig {
        servers: HashMap::new(),
        safe_tools: HashMap::new(),
    };
    let mut warnings: Vec<String> = Vec::new();

    // 1. 加载全局配置
    if let Some(global_path) = global_config_path {
        match load_config_file(global_path) {
            Ok(file) => merge_entries(&mut merged, file, &mut warnings),
            Err(e) => {
                // 全局配置文件不存在属于正常情况，静默跳过
                if !e.contains("not found") && !e.contains("NotFound") && !e.contains("找不到") {
                    warnings.push(format!("读取全局 MCP 配置失败 ({}): {}", global_path.display(), e));
                }
            }
        }
    }

    // 2. 加载项目级配置 (xuflow.json 中的 mcpServers)
    if let Some(project) = project_root {
        let project_config_path = project.join("xuflow.json");
        match load_config_file(&project_config_path) {
            Ok(file) => merge_entries(&mut merged, file, &mut warnings),
            Err(e) => {
                if !e.contains("not found") && !e.contains("NotFound") && !e.contains("找不到") {
                    warnings.push(format!(
                        "读取项目 MCP 配置失败 ({}): {}",
                        project_config_path.display(),
                        e
                    ));
                }
            }
        }
    }

    (merged, warnings)
}

/// 读取并反序列化一个配置文件
fn load_config_file(path: &Path) -> Result<McpServersFile, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("无法读取文件: {}", e))?;
    let config: McpServersFile =
        serde_json::from_str(&content).map_err(|e| format!("JSON 解析失败: {}", e))?;
    Ok(config)
}

/// 将配置项合并到运行时配置中
fn merge_entries(merged: &mut MergedMcpConfig, file: McpServersFile, warnings: &mut Vec<String>) {
    for (name, entry) in file.mcp_servers {
        if entry.disabled {
            continue; // 禁用的 Server 直接跳过
        }

        // 项目级覆盖全局同名 Server
        if merged.servers.contains_key(&name) {
            warnings.push(format!(
                "MCP Server '{}' 被项目级配置覆盖",
                name
            ));
        }

        merged.servers.insert(
            name.clone(),
            McpServerConfig {
                command: entry.command,
                args: entry.args,
                env: entry.env,
            },
        );
        merged.safe_tools.insert(name, entry.safe_tools);
    }
}

/// 解析全局配置文件的最终路径，优先级：
///   1. 环境变量 XUFLOW_MCP_CONFIG（指向具体文件路径）
///   2. 操作系统默认路径: %APPDATA%/xuflow/mcp_servers.json 或 ~/.config/xuflow/mcp_servers.json
pub fn resolve_global_config_path() -> Option<PathBuf> {
    // 环境变量优先 — 允许用户将配置放在任意磁盘位置
    if let Ok(custom_path) = std::env::var(ENV_MCP_CONFIG_PATH) {
        let p = PathBuf::from(custom_path);
        if p.exists() {
            return Some(p);
        }
        // 环境变量指向的文件不存在时也返回该路径，让上层报告具体的"文件不存在"错误
        // 这样用户能明确知道是路径配错了，而不是静默 fallback 到默认路径
        return Some(p);
    }

    default_global_config_path()
}

/// 获取全局配置文件的默认路径
pub fn default_global_config_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .ok()
            .map(|p| PathBuf::from(p).join("xuflow").join("mcp_servers.json"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        dirs::config_dir().map(|p| p.join("xuflow").join("mcp_servers.json"))
    }
}

/// 加载 MCP 配置，合并所有来源：环境变量 → 全局默认 → 项目级 → 显式自定义
/// explicit_path: 显式传入的路径（命令行参数或前端设置），优先级最高
/// project_root: 项目根目录，用于查找 xuflow.json
pub fn load_mcp_config_multi(
    explicit_path: Option<&Path>,
    project_root: Option<&Path>,
) -> (MergedMcpConfig, Vec<String>) {
    // 显式路径 > 环境变量 > 默认全局路径
    let global_path = match explicit_path {
        Some(p) => Some(p.to_path_buf()),
        None => resolve_global_config_path(),
    };
    load_mcp_config(global_path.as_deref(), project_root)
}
