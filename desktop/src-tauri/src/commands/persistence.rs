// SQLite 持久化模块 —— 替代 MySQL，启动即用，零配置。
// 底层使用 xuflow_core::SessionStore（rusqlite），Tauri 命令通过 spawn_blocking 异步调用。

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tauri::State;
use xuflow_core::SessionStore;

// ── 数据结构（与前端 TypeScript 类型字段名对应）──────────────────
// 直接 re-export core 中的类型，前端通过 invoke 获得的 JSON 字段名不变。

pub use xuflow_core::memory::session::{ProjectRow, SessionRow, MessageRow};

// ── 连接配置（保留结构体以兼容旧前端调用，SQLite 模式下忽略所有字段）──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MySqlOpts {
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub port: u16,
    #[serde(default)]
    pub user: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub database: String,
}

// ── 应用状态 ─────────────────────────────────────────────────

/// 数据库状态：持有 SessionStore 的 Arc，供 spawn_blocking 闭包克隆。
pub struct DbState {
    pub store: Arc<SessionStore>,
}

// ── 数据库路径查询 ─────────────────────────────────────────

/// 返回 SQLite 数据库文件的完整路径，供前端设置页显示。
#[tauri::command]
pub async fn db_get_path(
    state: State<'_, Arc<DbState>>,
) -> Result<String, String> {
    Ok(state.store.db_path())
}

// ── 连接管理命令（适配前端已有的调用）──────────────────────────

/// 初始化 SQLite 数据库连接。前端设置页的"保存并连接"按钮调用。
/// SQLite 模式下始终成功，opts 参数保留以兼容前端调用。
#[tauri::command]
pub async fn db_connect(
    _opts: MySqlOpts,
    state: State<'_, Arc<DbState>>,
) -> Result<bool, String> {
    // SQLite 在启动时已初始化，再次调用直接返回成功
    let _ = state; // 保持 state borrow
    Ok(true)
}

/// 测试连接 —— SQLite 模式下始终成功。
#[tauri::command]
pub async fn db_test_connection(_opts: MySqlOpts) -> Result<bool, String> {
    Ok(true)
}

/// 断开连接 —— SQLite 模式下为 no-op。
#[tauri::command]
pub async fn db_disconnect() -> Result<bool, String> {
    Ok(true)
}

/// 数据库是否已连接 —— SQLite 模式下始终为 true。
#[tauri::command]
pub async fn db_is_connected() -> Result<bool, String> {
    Ok(true)
}

// ── 项目 CRUD ────────────────────────────────────────────────

#[tauri::command]
pub async fn db_create_project(
    name: String,
    source: Option<String>,
    state: State<'_, Arc<DbState>>,
) -> Result<ProjectRow, String> {
    let store = state.store.clone();
    let source = source.unwrap_or_else(|| "local".to_string());
    let id = uid();

    tokio::task::spawn_blocking(move || {
        store.create_project(&id, &name, &source)
    })
    .await
    .map_err(|e| format!("线程错误：{}", e))?
    .map_err(|e| format!("创建项目失败：{}", e))
}

#[tauri::command]
pub async fn db_list_projects(
    state: State<'_, Arc<DbState>>,
) -> Result<Vec<ProjectRow>, String> {
    let store = state.store.clone();
    tokio::task::spawn_blocking(move || store.list_projects())
        .await
        .map_err(|e| format!("线程错误：{}", e))?
        .map_err(|e| format!("查询项目列表失败：{}", e))
}

#[tauri::command]
pub async fn db_update_project_name(
    id: String,
    name: String,
    state: State<'_, Arc<DbState>>,
) -> Result<bool, String> {
    let store = state.store.clone();
    tokio::task::spawn_blocking(move || store.update_project_name(&id, &name))
        .await
        .map_err(|e| format!("线程错误：{}", e))?
        .map_err(|e| format!("重命名项目失败：{}", e))
}

#[tauri::command]
pub async fn db_delete_project(
    id: String,
    state: State<'_, Arc<DbState>>,
) -> Result<bool, String> {
    let store = state.store.clone();
    tokio::task::spawn_blocking(move || store.delete_project(&id))
        .await
        .map_err(|e| format!("线程错误：{}", e))?
        .map_err(|e| format!("删除项目失败：{}", e))
}

// ── 会话 CRUD ────────────────────────────────────────────────

#[tauri::command]
pub async fn db_create_session(
    project_id: String,
    title: String,
    title_source: Option<String>,
    visible: Option<bool>,
    state: State<'_, Arc<DbState>>,
) -> Result<SessionRow, String> {
    let store = state.store.clone();
    let id = uid();
    let ts = title_source.unwrap_or_else(|| "default".to_string());
    let vis = visible.unwrap_or(true);

    tokio::task::spawn_blocking(move || {
        store.create_session(&id, &project_id, &title, &ts, vis)
    })
    .await
    .map_err(|e| format!("线程错误：{}", e))?
    .map_err(|e| format!("创建会话失败：{}", e))
}

#[tauri::command]
pub async fn db_list_sessions(
    project_id: String,
    state: State<'_, Arc<DbState>>,
) -> Result<Vec<SessionRow>, String> {
    let store = state.store.clone();
    tokio::task::spawn_blocking(move || store.list_sessions_by_project(&project_id))
        .await
        .map_err(|e| format!("线程错误：{}", e))?
        .map_err(|e| format!("查询会话列表失败：{}", e))
}

#[tauri::command]
pub async fn db_list_all_sessions(
    state: State<'_, Arc<DbState>>,
) -> Result<Vec<SessionRow>, String> {
    let store = state.store.clone();
    tokio::task::spawn_blocking(move || store.list_all_sessions())
        .await
        .map_err(|e| format!("线程错误：{}", e))?
        .map_err(|e| format!("查询全部会话失败：{}", e))
}

#[tauri::command]
pub async fn db_update_session_title(
    id: String,
    title: String,
    state: State<'_, Arc<DbState>>,
) -> Result<bool, String> {
    let store = state.store.clone();
    tokio::task::spawn_blocking(move || store.update_session_title(&id, &title))
        .await
        .map_err(|e| format!("线程错误：{}", e))?
        .map_err(|e| format!("重命名会话失败：{}", e))
}

#[tauri::command]
pub async fn db_delete_session(
    id: String,
    state: State<'_, Arc<DbState>>,
) -> Result<bool, String> {
    let store = state.store.clone();
    tokio::task::spawn_blocking(move || store.delete_session(&id))
        .await
        .map_err(|e| format!("线程错误：{}", e))?
        .map_err(|e| format!("删除会话失败：{}", e))
}

#[tauri::command]
pub async fn db_reveal_session(
    id: String,
    state: State<'_, Arc<DbState>>,
) -> Result<bool, String> {
    let store = state.store.clone();
    tokio::task::spawn_blocking(move || store.reveal_session(&id))
        .await
        .map_err(|e| format!("线程错误：{}", e))?
        .map_err(|e| format!("更新会话可见性失败：{}", e))
}

// ── 消息 CRUD ────────────────────────────────────────────────

#[tauri::command]
pub async fn db_add_message(
    session_id: String,
    role: String,
    content: String,
    done: Option<bool>,
    reasoning: Option<String>,
    tool_calls: Option<String>,
    state: State<'_, Arc<DbState>>,
) -> Result<MessageRow, String> {
    let store = state.store.clone();
    tokio::task::spawn_blocking(move || {
        store.add_message(
            &session_id,
            &role,
            &content,
            done.unwrap_or(false),
            reasoning.as_deref(),
            tool_calls.as_deref(),
        )
    })
    .await
    .map_err(|e| format!("线程错误：{}", e))?
    .map_err(|e| format!("添加消息失败：{}", e))
}

#[tauri::command]
pub async fn db_update_message(
    id: i64,
    fields_json: String,
    state: State<'_, Arc<DbState>>,
) -> Result<bool, String> {
    let store = state.store.clone();
    tokio::task::spawn_blocking(move || store.update_message(id, &fields_json))
        .await
        .map_err(|e| format!("线程错误：{}", e))?
        .map_err(|e| format!("更新消息失败：{}", e))
}

#[tauri::command]
pub async fn db_get_messages(
    session_id: String,
    state: State<'_, Arc<DbState>>,
) -> Result<Vec<MessageRow>, String> {
    let store = state.store.clone();
    tokio::task::spawn_blocking(move || store.get_messages(&session_id))
        .await
        .map_err(|e| format!("线程错误：{}", e))?
        .map_err(|e| format!("查询消息失败：{}", e))
}

#[tauri::command]
pub async fn db_clear_messages(
    session_id: String,
    state: State<'_, Arc<DbState>>,
) -> Result<u64, String> {
    let store = state.store.clone();
    tokio::task::spawn_blocking(move || store.clear_messages(&session_id))
        .await
        .map_err(|e| format!("线程错误：{}", e))?
        .map_err(|e| format!("清空消息失败：{}", e))
}

// ── 配置读写 ────────────────────────────────────────────────

#[tauri::command]
pub async fn db_get_config(
    key: String,
    state: State<'_, Arc<DbState>>,
) -> Result<Option<String>, String> {
    let store = state.store.clone();
    tokio::task::spawn_blocking(move || store.get_config(&key))
        .await
        .map_err(|e| format!("线程错误：{}", e))?
        .map_err(|e| format!("读取配置失败：{}", e))
}

#[tauri::command]
pub async fn db_set_config(
    key: String,
    value: String,
    state: State<'_, Arc<DbState>>,
) -> Result<bool, String> {
    let store = state.store.clone();
    tokio::task::spawn_blocking(move || store.set_config(&key, &value))
        .await
        .map_err(|e| format!("线程错误：{}", e))?
        .map_err(|e| format!("写入配置失败：{}", e))?;
    Ok(true)
}

#[tauri::command]
pub async fn db_delete_config(
    key: String,
    state: State<'_, Arc<DbState>>,
) -> Result<bool, String> {
    let store = state.store.clone();
    tokio::task::spawn_blocking(move || store.delete_config(&key))
        .await
        .map_err(|e| format!("线程错误：{}", e))?
        .map_err(|e| format!("删除配置失败：{}", e))
}

// ── 数据迁移 ────────────────────────────────────────────────

#[tauri::command]
pub async fn db_migrate_from_localstorage(
    frontend_projects_json: String,
    state: State<'_, Arc<DbState>>,
) -> Result<u32, String> {
    let store = state.store.clone();
    tokio::task::spawn_blocking(move || store.migrate_from_localstorage(&frontend_projects_json))
        .await
        .map_err(|e| format!("线程错误：{}", e))?
        .map_err(|e| format!("迁移数据失败：{}", e))
}

#[tauri::command]
pub async fn db_is_migrated(
    state: State<'_, Arc<DbState>>,
) -> Result<bool, String> {
    let store = state.store.clone();
    tokio::task::spawn_blocking(move || store.is_migrated())
        .await
        .map_err(|e| format!("线程错误：{}", e))?
        .map_err(|e| format!("查询迁移状态失败：{}", e))
}

// ── 工具函数 ────────────────────────────────────────────────

fn uid() -> String {
    use std::sync::atomic::AtomicU64;
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let seq = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{}-{}", ts, seq)
}
