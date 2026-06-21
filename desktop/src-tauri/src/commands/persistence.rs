// MySQL 持久化模块 — 替代前端 localStorage，所有项目和会话数据存入 MySQL。
// 使用 sqlx 异步连接池，通过 Tauri 命令暴露给前端 invoke() 调用。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlPool;
use sqlx::Row;
use tauri::State;
use tokio::sync::RwLock;

// ── 连接配置 ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MySqlOpts {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_user")]
    pub user: String,
    #[serde(default)]
    pub password: String,
    #[serde(default = "default_database")]
    pub database: String,
}

fn default_host() -> String { "127.0.0.1".into() }
fn default_port() -> u16 { 3306 }
fn default_user() -> String { "root".into() }
fn default_database() -> String { "xuflow".into() }

// ── 数据行结构（与前端 TypeScript 类型对应）──────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRow {
    pub id: String,
    pub name: String,
    pub path: Option<String>,
    pub source: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRow {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub title_source: String,
    pub visible: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRow {
    pub id: i64,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub done: bool,
    pub reasoning: Option<String>,
    pub reasoning_done: bool,
    pub tool_calls: Option<String>,
    pub created_at: i64,
}

// ── 应用状态 ─────────────────────────────────────────────────

pub struct DbState {
    pub pool: RwLock<Option<MySqlPool>>,
    pub connected: AtomicBool,
}

impl DbState {
    pub fn new() -> Self {
        Self {
            pool: RwLock::new(None),
            connected: AtomicBool::new(false),
        }
    }

    /// 获取连接池引用，如果未连接则返回错误。
    async fn pool(&self) -> Result<MySqlPool, String> {
        let guard = self.pool.read().await;
        guard.clone().ok_or_else(|| "数据库未连接，请先在设置中配置 MySQL 连接".to_string())
    }
}

// ── 数据库建表 ───────────────────────────────────────────────

async fn init_schema(pool: &MySqlPool) -> Result<(), sqlx::Error> {
    // 项目表
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS projects (
            id VARCHAR(64) NOT NULL,
            name VARCHAR(255) NOT NULL,
            path VARCHAR(1024) NULL,
            source VARCHAR(32) NOT NULL DEFAULT 'local',
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL,
            PRIMARY KEY (id)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci"
    ).execute(pool).await?;

    // 会话表
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS sessions (
            id VARCHAR(64) NOT NULL,
            project_id VARCHAR(64) NOT NULL,
            title VARCHAR(255) NOT NULL,
            title_source VARCHAR(16) NOT NULL DEFAULT 'default',
            visible TINYINT NOT NULL DEFAULT 1,
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL,
            PRIMARY KEY (id),
            INDEX idx_project_id (project_id),
            CONSTRAINT fk_sessions_project
                FOREIGN KEY (project_id) REFERENCES projects(id)
                ON DELETE CASCADE
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci"
    ).execute(pool).await?;

    // 消息表
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS messages (
            id BIGINT NOT NULL AUTO_INCREMENT,
            session_id VARCHAR(64) NOT NULL,
            role VARCHAR(16) NOT NULL,
            content LONGTEXT NOT NULL,
            done TINYINT NOT NULL DEFAULT 0,
            reasoning LONGTEXT NULL,
            reasoning_done TINYINT NOT NULL DEFAULT 0,
            tool_calls LONGTEXT NULL,
            created_at BIGINT NOT NULL,
            PRIMARY KEY (id),
            INDEX idx_messages_session (session_id),
            CONSTRAINT fk_messages_session
                FOREIGN KEY (session_id) REFERENCES sessions(id)
                ON DELETE CASCADE
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci"
    ).execute(pool).await?;

    // 配置表（key-value）
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS config (
            k VARCHAR(128) NOT NULL,
            v LONGTEXT NOT NULL,
            PRIMARY KEY (k)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci"
    ).execute(pool).await?;

    Ok(())
}

// ── 连接管理命令 ─────────────────────────────────────────────

/// 测试连接并保存配置，成功后初始化 schema。
/// 由设置页"保存并连接"按钮调用。
#[tauri::command]
pub async fn db_connect(
    opts: MySqlOpts,
    state: State<'_, Arc<DbState>>,
) -> Result<bool, String> {
    let url = format!(
        "mysql://{}:{}@{}:{}/{}",
        opts.user, opts.password, opts.host, opts.port, opts.database
    );

    let pool = MySqlPool::connect(&url)
        .await
        .map_err(|e| format!("MySQL 连接失败：{}", e))?;

    // 建表
    init_schema(&pool)
        .await
        .map_err(|e| format!("建表失败：{}", e))?;

    // 保存连接池
    let mut guard = state.pool.write().await;
    *guard = Some(pool);
    state.connected.store(true, Ordering::SeqCst);

    Ok(true)
}

/// 仅测试连接，不保存状态也不建表。
/// 由设置页"测试连接"按钮调用。
#[tauri::command]
pub async fn db_test_connection(opts: MySqlOpts) -> Result<bool, String> {
    let url = format!(
        "mysql://{}:{}@{}:{}/{}",
        opts.user, opts.password, opts.host, opts.port, opts.database
    );

    MySqlPool::connect(&url)
        .await
        .map_err(|e| format!("连接失败：{}", e))?;

    Ok(true)
}

/// 断开当前连接，清空连接池。
#[tauri::command]
pub async fn db_disconnect(state: State<'_, Arc<DbState>>) -> Result<bool, String> {
    let mut guard = state.pool.write().await;
    if let Some(pool) = guard.take() {
        pool.close().await;
    }
    state.connected.store(false, Ordering::SeqCst);
    Ok(true)
}

/// 检查当前是否已连接。
#[tauri::command]
pub async fn db_is_connected(state: State<'_, Arc<DbState>>) -> Result<bool, String> {
    Ok(state.connected.load(Ordering::SeqCst))
}

// ── 项目 CRUD ────────────────────────────────────────────────

/// 创建项目，返回完整的 ProjectRow。
#[tauri::command]
pub async fn db_create_project(
    name: String,
    source: Option<String>,
    state: State<'_, Arc<DbState>>,
) -> Result<ProjectRow, String> {
    let pool = state.pool().await?;
    let id = uid();
    let source = source.unwrap_or_else(|| "local".to_string());
    let now = ts_now();

    let row = sqlx::query(
        "INSERT INTO projects (id, name, source, created_at, updated_at) VALUES (?, ?, ?, ?, ?)"
    )
        .bind(&id)
        .bind(&name)
        .bind(&source)
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .map_err(|e| format!("创建项目失败：{}", e))?;

    // 检查影响行数
    if row.rows_affected() == 0 {
        return Err("创建项目失败：未写入任何行".into());
    }

    Ok(ProjectRow {
        id,
        name,
        path: None,
        source,
        created_at: now,
        updated_at: now,
    })
}

/// 列出所有项目（按更新时间倒序）。
#[tauri::command]
pub async fn db_list_projects(
    state: State<'_, Arc<DbState>>,
) -> Result<Vec<ProjectRow>, String> {
    let pool = state.pool().await?;

    let rows = sqlx::query(
        "SELECT id, name, path, source, created_at, updated_at FROM projects ORDER BY updated_at DESC"
    )
        .fetch_all(&pool)
        .await
        .map_err(|e| format!("查询项目列表失败：{}", e))?;

    let projects = rows
        .iter()
        .map(|r| ProjectRow {
            id: r.get("id"),
            name: r.get("name"),
            path: r.get("path"),
            source: r.get("source"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        })
        .collect();

    Ok(projects)
}

/// 重命名项目。
#[tauri::command]
pub async fn db_update_project_name(
    id: String,
    name: String,
    state: State<'_, Arc<DbState>>,
) -> Result<bool, String> {
    let pool = state.pool().await?;
    let now = ts_now();

    let row = sqlx::query(
        "UPDATE projects SET name = ?, updated_at = ? WHERE id = ?"
    )
        .bind(&name)
        .bind(now)
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|e| format!("重命名项目失败：{}", e))?;

    Ok(row.rows_affected() > 0)
}

/// 删除项目（级联删除其下所有会话和消息）。
#[tauri::command]
pub async fn db_delete_project(
    id: String,
    state: State<'_, Arc<DbState>>,
) -> Result<bool, String> {
    let pool = state.pool().await?;

    let row = sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|e| format!("删除项目失败：{}", e))?;

    Ok(row.rows_affected() > 0)
}

// ── 会话 CRUD ────────────────────────────────────────────────

/// 创建会话，返回完整的 SessionRow。
#[tauri::command]
pub async fn db_create_session(
    project_id: String,
    title: String,
    title_source: Option<String>,
    visible: Option<bool>,
    state: State<'_, Arc<DbState>>,
) -> Result<SessionRow, String> {
    let pool = state.pool().await?;
    let id = uid();
    let title_source = title_source.unwrap_or_else(|| "default".to_string());
    let visible = visible.unwrap_or(true);
    let visible_int: i8 = if visible { 1 } else { 0 };
    let now = ts_now();

    sqlx::query(
        "INSERT INTO sessions (id, project_id, title, title_source, visible, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
        .bind(&id)
        .bind(&project_id)
        .bind(&title)
        .bind(&title_source)
        .bind(visible_int)
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .map_err(|e| format!("创建会话失败：{}", e))?;

    // 同时更新项目的 updated_at
    sqlx::query("UPDATE projects SET updated_at = ? WHERE id = ?")
        .bind(now)
        .bind(&project_id)
        .execute(&pool)
        .await
        .map_err(|e| format!("更新项目时间失败：{}", e))?;

    Ok(SessionRow {
        id,
        project_id,
        title,
        title_source,
        visible,
        created_at: now,
        updated_at: now,
    })
}

/// 列出指定项目下的所有会话（按更新时间倒序）。
#[tauri::command]
pub async fn db_list_sessions(
    project_id: String,
    state: State<'_, Arc<DbState>>,
) -> Result<Vec<SessionRow>, String> {
    let pool = state.pool().await?;

    let rows = sqlx::query(
        "SELECT id, project_id, title, title_source, visible, created_at, updated_at \
         FROM sessions WHERE project_id = ? ORDER BY updated_at DESC"
    )
        .bind(&project_id)
        .fetch_all(&pool)
        .await
        .map_err(|e| format!("查询会话列表失败：{}", e))?;

    let sessions = rows
        .iter()
        .map(|r| {
            let visible_int: i8 = r.get("visible");
            SessionRow {
                id: r.get("id"),
                project_id: r.get("project_id"),
                title: r.get("title"),
                title_source: r.get("title_source"),
                visible: visible_int != 0,
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            }
        })
        .collect();

    Ok(sessions)
}

/// 列出所有会话（用于迁移/全局加载），不按项目过滤。
#[tauri::command]
pub async fn db_list_all_sessions(
    state: State<'_, Arc<DbState>>,
) -> Result<Vec<SessionRow>, String> {
    let pool = state.pool().await?;

    let rows = sqlx::query(
        "SELECT id, project_id, title, title_source, visible, created_at, updated_at \
         FROM sessions ORDER BY updated_at DESC"
    )
        .fetch_all(&pool)
        .await
        .map_err(|e| format!("查询全部会话失败：{}", e))?;

    let sessions = rows
        .iter()
        .map(|r| {
            let visible_int: i8 = r.get("visible");
            SessionRow {
                id: r.get("id"),
                project_id: r.get("project_id"),
                title: r.get("title"),
                title_source: r.get("title_source"),
                visible: visible_int != 0,
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            }
        })
        .collect();

    Ok(sessions)
}

/// 重命名会话，始终将 title_source 设为 'manual' 以防止自动摘要覆盖。
#[tauri::command]
pub async fn db_update_session_title(
    id: String,
    title: String,
    state: State<'_, Arc<DbState>>,
) -> Result<bool, String> {
    let pool = state.pool().await?;
    let now = ts_now();

    let row = sqlx::query(
        "UPDATE sessions SET title = ?, title_source = 'manual', updated_at = ? WHERE id = ?"
    )
        .bind(&title)
        .bind(now)
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|e| format!("重命名会话失败：{}", e))?;

    Ok(row.rows_affected() > 0)
}

/// 删除会话（级联删除其下所有消息）。
#[tauri::command]
pub async fn db_delete_session(
    id: String,
    state: State<'_, Arc<DbState>>,
) -> Result<bool, String> {
    let pool = state.pool().await?;

    let row = sqlx::query("DELETE FROM sessions WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|e| format!("删除会话失败：{}", e))?;

    Ok(row.rows_affected() > 0)
}

/// 将隐藏会话设为可见（首次 AI 响应完成后调用）。
#[tauri::command]
pub async fn db_reveal_session(
    id: String,
    state: State<'_, Arc<DbState>>,
) -> Result<bool, String> {
    let pool = state.pool().await?;
    let now = ts_now();

    let row = sqlx::query(
        "UPDATE sessions SET visible = 1, updated_at = ? WHERE id = ?"
    )
        .bind(now)
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|e| format!("更新会话可见性失败：{}", e))?;

    Ok(row.rows_affected() > 0)
}

// ── 消息 CRUD ────────────────────────────────────────────────

/// 添加消息，返回包含自增 id 的 MessageRow。
#[tauri::command]
pub async fn db_add_message(
    session_id: String,
    role: String,
    content: String,
    reasoning: Option<String>,
    tool_calls: Option<String>,
    state: State<'_, Arc<DbState>>,
) -> Result<MessageRow, String> {
    let pool = state.pool().await?;
    let now = ts_now();

    let row = sqlx::query(
        "INSERT INTO messages (session_id, role, content, done, reasoning, reasoning_done, tool_calls, created_at) \
         VALUES (?, ?, ?, 0, ?, 0, ?, ?)"
    )
        .bind(&session_id)
        .bind(&role)
        .bind(&content)
        .bind(&reasoning)
        .bind(&tool_calls)
        .bind(now)
        .execute(&pool)
        .await
        .map_err(|e| format!("添加消息失败：{}", e))?;

    // 更新会话的 updated_at
    sqlx::query("UPDATE sessions SET updated_at = ? WHERE id = ?")
        .bind(now)
        .bind(&session_id)
        .execute(&pool)
        .await
        .map_err(|e| format!("更新会话时间失败：{}", e))?;

    let id = row.last_insert_id() as i64;

    Ok(MessageRow {
        id,
        session_id,
        role,
        content,
        done: false,
        reasoning,
        reasoning_done: false,
        tool_calls,
        created_at: now,
    })
}

/// 更新消息内容（用于流式 delta 更新和完成标记）。
/// fields_json 是一个 JSON 对象，包含要更新的字段：content, done, reasoning, reasoning_done, tool_calls
#[tauri::command]
pub async fn db_update_message(
    id: i64,
    fields_json: String,
    state: State<'_, Arc<DbState>>,
) -> Result<bool, String> {
    let pool = state.pool().await?;

    // 解析要更新的字段
    let fields: serde_json::Value = serde_json::from_str(&fields_json)
        .map_err(|e| format!("解析更新字段失败：{}", e))?;

    // 构建动态 UPDATE 语句
    let mut set_clauses = Vec::new();
    let mut bind_values: Vec<String> = Vec::new(); // 字符串参数

    if let Some(content) = fields.get("content").and_then(|v| v.as_str()) {
        set_clauses.push("content = ?");
        bind_values.push(content.to_string());
    }
    if let Some(done) = fields.get("done").and_then(|v| v.as_bool()) {
        set_clauses.push("done = ?");
        bind_values.push(if done { "1".into() } else { "0".into() });
    }
    if let Some(reasoning) = fields.get("reasoning").and_then(|v| v.as_str()) {
        set_clauses.push("reasoning = ?");
        bind_values.push(reasoning.to_string());
    }
    if let Some(rd) = fields.get("reasoning_done").and_then(|v| v.as_bool()) {
        set_clauses.push("reasoning_done = ?");
        bind_values.push(if rd { "1".into() } else { "0".into() });
    }
    if let Some(tool_calls) = fields.get("tool_calls") {
        set_clauses.push("tool_calls = ?");
        bind_values.push(serde_json::to_string(tool_calls).unwrap_or_default());
    }

    if set_clauses.is_empty() {
        return Ok(false);
    }

    let sql = format!(
        "UPDATE messages SET {} WHERE id = ?",
        set_clauses.join(", ")
    );

    // 构建查询
    let mut query = sqlx::query(&sql);
    for val in &bind_values {
        query = query.bind(val);
    }
    query = query.bind(id);

    query.execute(&pool)
        .await
        .map_err(|e| format!("更新消息失败：{}", e))?;

    Ok(true)
}

/// 获取指定会话的所有消息（按创建时间正序）。
#[tauri::command]
pub async fn db_get_messages(
    session_id: String,
    state: State<'_, Arc<DbState>>,
) -> Result<Vec<MessageRow>, String> {
    let pool = state.pool().await?;

    let rows = sqlx::query(
        "SELECT id, session_id, role, content, done, reasoning, reasoning_done, tool_calls, created_at \
         FROM messages WHERE session_id = ? ORDER BY id ASC"
    )
        .bind(&session_id)
        .fetch_all(&pool)
        .await
        .map_err(|e| format!("查询消息失败：{}", e))?;

    let messages = rows
        .iter()
        .map(|r| {
            let done_int: i8 = r.get("done");
            let rd_int: i8 = r.get("reasoning_done");
            MessageRow {
                id: r.get("id"),
                session_id: r.get("session_id"),
                role: r.get("role"),
                content: r.get("content"),
                done: done_int != 0,
                reasoning: r.get("reasoning"),
                reasoning_done: rd_int != 0,
                tool_calls: r.get("tool_calls"),
                created_at: r.get("created_at"),
            }
        })
        .collect();

    Ok(messages)
}

/// 清空指定会话的所有消息。
#[tauri::command]
pub async fn db_clear_messages(
    session_id: String,
    state: State<'_, Arc<DbState>>,
) -> Result<u64, String> {
    let pool = state.pool().await?;

    let row = sqlx::query("DELETE FROM messages WHERE session_id = ?")
        .bind(&session_id)
        .execute(&pool)
        .await
        .map_err(|e| format!("清空消息失败：{}", e))?;

    Ok(row.rows_affected())
}

// ── 配置读写 ────────────────────────────────────────────────

/// 读取配置值。
#[tauri::command]
pub async fn db_get_config(
    key: String,
    state: State<'_, Arc<DbState>>,
) -> Result<Option<String>, String> {
    let pool = state.pool().await?;

    let row = sqlx::query("SELECT v FROM config WHERE k = ?")
        .bind(&key)
        .fetch_optional(&pool)
        .await
        .map_err(|e| format!("读取配置失败：{}", e))?;

    Ok(row.map(|r| r.get("v")))
}

/// 写入配置值（UPSERT）。
#[tauri::command]
pub async fn db_set_config(
    key: String,
    value: String,
    state: State<'_, Arc<DbState>>,
) -> Result<bool, String> {
    let pool = state.pool().await?;

    sqlx::query(
        "INSERT INTO config (k, v) VALUES (?, ?) ON DUPLICATE KEY UPDATE v = VALUES(v)"
    )
        .bind(&key)
        .bind(&value)
        .execute(&pool)
        .await
        .map_err(|e| format!("写入配置失败：{}", e))?;

    Ok(true)
}

/// 删除配置项。
#[tauri::command]
pub async fn db_delete_config(
    key: String,
    state: State<'_, Arc<DbState>>,
) -> Result<bool, String> {
    let pool = state.pool().await?;

    let row = sqlx::query("DELETE FROM config WHERE k = ?")
        .bind(&key)
        .execute(&pool)
        .await
        .map_err(|e| format!("删除配置失败：{}", e))?;

    Ok(row.rows_affected() > 0)
}

// ── 数据迁移 ────────────────────────────────────────────────

/// 从前端传入的 localStorage JSON 迁移旧数据到 MySQL。
/// frontend_projects_json: localStorage "xuflow-projects" 的完整 JSON 字符串
/// 返回迁移的消息总数。
#[tauri::command]
pub async fn db_migrate_from_localstorage(
    frontend_projects_json: String,
    state: State<'_, Arc<DbState>>,
) -> Result<u32, String> {
    let pool = state.pool().await?;

    // 解析 JSON
    let data: serde_json::Value = serde_json::from_str(&frontend_projects_json)
        .map_err(|e| format!("解析迁移数据失败：{}", e))?;

    let projects = data.get("projects")
        .and_then(|v| v.as_array())
        .ok_or("迁移数据中缺少 projects 数组")?;

    let mut total_messages: u32 = 0;

    for project in projects {
        let project_id = project.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let project_name = project.get("name").and_then(|v| v.as_str()).unwrap_or("未命名项目");
        let project_source = project.get("source").and_then(|v| v.as_str()).unwrap_or("local");
        let project_path = project.get("path").and_then(|v| v.as_str()).map(|s| s.to_string());
        let created_at = project.get("createdAt").and_then(|v| v.as_i64()).unwrap_or(0);
        let updated_at = project.get("updatedAt").and_then(|v| v.as_i64()).unwrap_or(0);

        // 插入项目（IGNORE 重复）
        sqlx::query(
            "INSERT IGNORE INTO projects (id, name, path, source, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)"
        )
            .bind(project_id)
            .bind(project_name)
            .bind(&project_path)
            .bind(project_source)
            .bind(created_at)
            .bind(updated_at)
            .execute(&pool)
            .await
            .map_err(|e| format!("迁移项目失败：{}", e))?;

        // 遍历会话
        let conversations = project.get("conversations")
            .and_then(|v| v.as_array());
        if let Some(convs) = conversations {
            for conv in convs {
                let conv_id = conv.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let conv_title = conv.get("title").and_then(|v| v.as_str()).unwrap_or("未命名会话");
                let title_source = conv.get("titleSource").and_then(|v| v.as_str()).unwrap_or("default");
                let visible = conv.get("visible").and_then(|v| v.as_bool()).unwrap_or(true);
                let visible_int: i8 = if visible { 1 } else { 0 };
                let conv_created = conv.get("createdAt").and_then(|v| v.as_i64()).unwrap_or(0);
                let conv_updated = conv.get("updatedAt").and_then(|v| v.as_i64()).unwrap_or(0);

                // 插入会话
                sqlx::query(
                    "INSERT IGNORE INTO sessions (id, project_id, title, title_source, visible, created_at, updated_at) \
                     VALUES (?, ?, ?, ?, ?, ?, ?)"
                )
                    .bind(conv_id)
                    .bind(project_id)
                    .bind(conv_title)
                    .bind(title_source)
                    .bind(visible_int)
                    .bind(conv_created)
                    .bind(conv_updated)
                    .execute(&pool)
                    .await
                    .map_err(|e| format!("迁移会话失败：{}", e))?;

                // 遍历消息
                let messages = conv.get("messages")
                    .and_then(|v| v.as_array());
                if let Some(msgs) = messages {
                    for msg in msgs {
                        let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("");
                        let content = msg.get("content").and_then(|v| v.as_str()).unwrap_or("");
                        let done = msg.get("done").and_then(|v| v.as_bool()).unwrap_or(false);
                        let done_int: i8 = if done { 1 } else { 0 };
                        let reasoning = msg.get("reasoning").and_then(|v| v.as_str()).map(|s| s.to_string());
                        let reasoning_done = msg.get("reasoningDone").and_then(|v| v.as_bool()).unwrap_or(false);
                        let rd_int: i8 = if reasoning_done { 1 } else { 0 };
                        let tool_calls = msg.get("toolCalls").map(|v| v.to_string());
                        let msg_ts = None; // 不强制时间戳

                        // 插入消息
                        sqlx::query(
                            "INSERT INTO messages (session_id, role, content, done, reasoning, reasoning_done, tool_calls, created_at) \
                             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
                        )
                            .bind(conv_id)
                            .bind(role)
                            .bind(content)
                            .bind(done_int)
                            .bind(&reasoning)
                            .bind(rd_int)
                            .bind(&tool_calls)
                            .bind(msg_ts.unwrap_or(ts_now()))
                            .execute(&pool)
                            .await
                            .map_err(|e| format!("迁移消息失败：{}", e))?;
                        total_messages += 1;
                    }
                }
            }
        }
    }

    // 标记迁移完成
    sqlx::query(
        "INSERT INTO config (k, v) VALUES ('migrated', 'true') ON DUPLICATE KEY UPDATE v = 'true'"
    )
        .execute(&pool)
        .await
        .map_err(|e| format!("写入迁移标记失败：{}", e))?;

    Ok(total_messages)
}

/// 检查是否已经迁移过。
#[tauri::command]
pub async fn db_is_migrated(
    state: State<'_, Arc<DbState>>,
) -> Result<bool, String> {
    let pool = state.pool().await?;

    let row = sqlx::query("SELECT v FROM config WHERE k = 'migrated'")
        .fetch_optional(&pool)
        .await
        .map_err(|e| format!("查询迁移状态失败：{}", e))?;

    Ok(row.map(|r| r.get::<String, _>("v")) == Some("true".to_string()))
}

// ── 工具函数 ────────────────────────────────────────────────

/// 生成唯一 ID（时间戳 + 自增计数器）。
fn uid() -> String {
    use std::sync::atomic::AtomicU64;
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{}", ts, seq)
}

/// 当前时间戳（毫秒级）。
fn ts_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}
