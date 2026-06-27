// Stdio Transport — 本地子进程 JSON-RPC 通信
// spawn MCP Server 子进程，stdin 写请求、stdout 逐行读响应
// 后台 tokio task 持续监听 stdout，通过 oneshot channel 将响应路由给对应请求

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{oneshot, Mutex};

use super::McpTransport;
use crate::mcp::types::{JsonRpcRequest, JsonRpcResponse, McpError};

/// Stdio transport 的共享状态，由后台 tokio task 和 send() 双方访问
struct SharedState {
    alive: AtomicBool,
    pending: Mutex<HashMap<u64, oneshot::Sender<Result<JsonRpcResponse, McpError>>>>,
}

/// Stdio transport — 管理一个 MCP Server 子进程的完整生命周期
pub struct StdioTransport {
    /// 子进程 stdin 写入句柄，发送 JSON-RPC 请求时使用
    stdin: Mutex<ChildStdin>,
    /// 共享状态（Arc 包装），允许后台 task 和 send() 并发访问
    shared: Arc<SharedState>,
    /// 持有子进程句柄，析构时自动 kill
    _child: Mutex<Child>,
}

impl StdioTransport {
    /// 创建并启动一个新的 MCP Server 子进程，启动后台 stdout 监听
    pub async fn spawn(
        command: &str,
        args: &[String],
        env: Option<&HashMap<String, String>>,
    ) -> Result<Self, McpError> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .kill_on_drop(true);

        // 注入环境变量（如 API Key）
        if let Some(env_vars) = env {
            for (k, v) in env_vars {
                cmd.env(k, v);
            }
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| McpError::Transport(format!("无法启动子进程 '{}': {}", command, e)))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| McpError::Transport("无法获取子进程 stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| McpError::Transport("无法获取子进程 stdout".into()))?;

        let shared = Arc::new(SharedState {
            alive: AtomicBool::new(true),
            pending: Mutex::new(HashMap::new()),
        });

        // 启动后台任务，持续读取子进程 stdout 中的 JSON-RPC 响应
        spawn_reader(stdout, Arc::clone(&shared));

        Ok(Self {
            stdin: Mutex::new(stdin),
            shared,
            _child: Mutex::new(child),
        })
    }
}

/// 后台 tokio task：逐行读取 stdout 并路由响应到对应请求的 oneshot sender
fn spawn_reader(stdout: ChildStdout, shared: Arc<SharedState>) {
    let reader = BufReader::new(stdout);
    tokio::spawn(async move {
        let mut lines = reader.lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    let line = line.trim().to_string();
                    if line.is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<JsonRpcResponse>(&line) {
                        Ok(response) => {
                            let id = response.id;
                            let mut pending_guard = shared.pending.lock().await;
                            if let Some(sender) = pending_guard.remove(&id.unwrap_or(0)) {
                                let _ = sender.send(Ok(response));
                            }
                        }
                        Err(e) => {
                            let mut pending_guard = shared.pending.lock().await;
                            let err = McpError::Protocol(format!(
                                "无法解析 MCP 响应: {} —— 原始数据: {}",
                                e,
                                &line[..line.len().min(200)]
                            ));
                            for (_, sender) in pending_guard.drain() {
                                let _ = sender.send(Err(err.clone()));
                            }
                            shared.alive.store(false, Ordering::SeqCst);
                            return;
                        }
                    }
                }
                Ok(None) => {
                    // stdout EOF — 子进程已退出
                    let mut pending_guard = shared.pending.lock().await;
                    let err = McpError::Transport("MCP Server 进程已退出".into());
                    for (_, sender) in pending_guard.drain() {
                        let _ = sender.send(Err(err.clone()));
                    }
                    shared.alive.store(false, Ordering::SeqCst);
                    return;
                }
                Err(e) => {
                    let mut pending_guard = shared.pending.lock().await;
                    let err = McpError::Transport(format!("读取 MCP Server 输出失败: {}", e));
                    for (_, sender) in pending_guard.drain() {
                        let _ = sender.send(Err(err.clone()));
                    }
                    shared.alive.store(false, Ordering::SeqCst);
                    return;
                }
            }
        }
    });
}

#[async_trait::async_trait]
impl McpTransport for StdioTransport {
    async fn send(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
        if !self.shared.alive.load(Ordering::SeqCst) {
            return Err(McpError::Transport("MCP Server 已断开".into()));
        }

        let id = request.id;
        let (tx, rx) = oneshot::channel();

        // 注册 pending 请求，后台读 task 根据 id 路由响应
        {
            let mut pending = self.shared.pending.lock().await;
            pending.insert(id, tx);
        }

        // 序列化并写入 stdin，每条 JSON 一行
        let json_str =
            serde_json::to_string(&request).map_err(|e| McpError::Protocol(e.to_string()))?;

        {
            let mut stdin = self.stdin.lock().await;
            stdin
                .write_all(json_str.as_bytes())
                .await
                .map_err(|e| McpError::Transport(format!("写入 stdin 失败: {}", e)))?;
            stdin
                .write_all(b"\n")
                .await
                .map_err(|e| McpError::Transport(format!("写入 stdin 换行失败: {}", e)))?;
            stdin
                .flush()
                .await
                .map_err(|e| McpError::Transport(format!("flush stdin 失败: {}", e)))?;
        }

        // 等待后台读 task 返回响应，设置 60 秒超时
        match tokio::time::timeout(std::time::Duration::from_secs(60), rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => {
                self.shared.alive.store(false, Ordering::SeqCst);
                Err(McpError::Transport("MCP Server 连接意外断开".into()))
            }
            Err(_) => {
                let mut pending = self.shared.pending.lock().await;
                pending.remove(&id);
                Err(McpError::Timeout(60))
            }
        }
    }

    fn is_alive(&self) -> bool {
        self.shared.alive.load(Ordering::SeqCst)
    }

    async fn close(&self) -> Result<(), McpError> {
        self.shared.alive.store(false, Ordering::SeqCst);

        let mut pending = self.shared.pending.lock().await;
        let err = McpError::Transport("MCP 连接已关闭".into());
        for (_, sender) in pending.drain() {
            let _ = sender.send(Err(err.clone()));
        }

        Ok(())
    }
}
