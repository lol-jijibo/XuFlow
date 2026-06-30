/// SQLite 持久化层 —— 替代 MySQL，桌面端零配置本地文件存储。
/// 使用 rusqlite 同步 API，外层由 Tauri command 通过 spawn_blocking 调用。
///
/// 数据文件默认路径（跨平台）：
///   Windows: %APPDATA%/xuflow/xuflow.db
///   macOS:   ~/Library/Application Support/xuflow/xuflow.db
///   Linux:   ~/.local/share/xuflow/xuflow.db
use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

// ── 数据行结构（与前端 TypeScript 类型字段名对应）──────────────────

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

// ── 兼容旧版结构体（保留原有公开类型名）─────────────────────────

/// 旧版 Session（仅 id/title/timestamps），保留以兼容外部引用。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

/// 旧版 Message（仅基础字段），保留以兼容外部引用。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: i64,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

// ── 数据库实例 ─────────────────────────────────────────────────

pub struct SessionStore {
    conn: Mutex<Connection>,
    // 数据库文件的实际路径，方便前端查询。
    path: PathBuf,
}

impl SessionStore {
    /// 打开 SQLite 数据库，自动建表。
    /// `db_path` 为 None 时使用系统默认的 app data 目录。
    pub fn open(db_path: Option<PathBuf>) -> Result<Self> {
        let path = db_path.unwrap_or_else(default_data_dir);

        // 确保父目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("无法创建数据目录: {:?}", parent))?;
        }

        let conn = Connection::open(&path)
            .with_context(|| format!("无法打开数据库: {:?}", path))?;

        // 启用外键约束和 WAL 模式以提升并发性能
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;",
        )?;

        Self::init_schema(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
            path,
        })
    }

    /// 建表（CREATE TABLE IF NOT EXISTS，幂等）。
    fn init_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                path TEXT,
                source TEXT NOT NULL DEFAULT 'local',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                title TEXT NOT NULL,
                title_source TEXT NOT NULL DEFAULT 'default',
                visible INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_sessions_project
                ON sessions(project_id);

            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                done INTEGER NOT NULL DEFAULT 0,
                reasoning TEXT,
                reasoning_done INTEGER NOT NULL DEFAULT 0,
                tool_calls TEXT,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_messages_session
                ON messages(session_id);

            CREATE TABLE IF NOT EXISTS config (
                k TEXT PRIMARY KEY,
                v TEXT NOT NULL
            );",
        )?;
        Ok(())
    }

    /// 返回数据库文件的完整路径，供前端设置页显示。
    pub fn db_path(&self) -> String {
        self.path.to_string_lossy().to_string()
    }

    // ── 项目 CRUD ───────────────────────────────────────────

    pub fn create_project(&self, id: &str, name: &str, source: &str) -> Result<ProjectRow> {
        let conn = self.conn.lock().unwrap();
        let now = ts_now();
        conn.execute(
            "INSERT INTO projects (id, name, source, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, name, source, now, now],
        )?;
        Ok(ProjectRow {
            id: id.to_string(),
            name: name.to_string(),
            path: None,
            source: source.to_string(),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn list_projects(&self) -> Result<Vec<ProjectRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, path, source, created_at, updated_at FROM projects ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ProjectRow {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                source: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn update_project_name(&self, id: &str, name: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let now = ts_now();
        let affected = conn.execute(
            "UPDATE projects SET name = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![name, now, id],
        )?;
        Ok(affected > 0)
    }

    pub fn delete_project(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM projects WHERE id = ?1", rusqlite::params![id])?;
        Ok(affected > 0)
    }

    // ── 会话 CRUD ───────────────────────────────────────────

    pub fn create_session(
        &self,
        id: &str,
        project_id: &str,
        title: &str,
        title_source: &str,
        visible: bool,
    ) -> Result<SessionRow> {
        let conn = self.conn.lock().unwrap();
        let now = ts_now();
        let vis: i32 = if visible { 1 } else { 0 };
        conn.execute(
            "INSERT INTO sessions (id, project_id, title, title_source, visible, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![id, project_id, title, title_source, vis, now, now],
        )?;
        // 更新项目时间
        conn.execute(
            "UPDATE projects SET updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, project_id],
        )?;
        Ok(SessionRow {
            id: id.to_string(),
            project_id: project_id.to_string(),
            title: title.to_string(),
            title_source: title_source.to_string(),
            visible,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn list_sessions_by_project(&self, project_id: &str) -> Result<Vec<SessionRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, title, title_source, visible, created_at, updated_at \
             FROM sessions WHERE project_id = ?1 ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map(rusqlite::params![project_id], |row| {
            let vis: i32 = row.get(4)?;
            Ok(SessionRow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                title: row.get(2)?,
                title_source: row.get(3)?,
                visible: vis != 0,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn list_all_sessions(&self) -> Result<Vec<SessionRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, title, title_source, visible, created_at, updated_at \
             FROM sessions ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let vis: i32 = row.get(4)?;
            Ok(SessionRow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                title: row.get(2)?,
                title_source: row.get(3)?,
                visible: vis != 0,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn update_session_title(&self, id: &str, title: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let now = ts_now();
        let affected = conn.execute(
            "UPDATE sessions SET title = ?1, title_source = 'manual', updated_at = ?2 WHERE id = ?3",
            rusqlite::params![title, now, id],
        )?;
        Ok(affected > 0)
    }

    pub fn delete_session(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM sessions WHERE id = ?1", rusqlite::params![id])?;
        Ok(affected > 0)
    }

    pub fn reveal_session(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let now = ts_now();
        let affected = conn.execute(
            "UPDATE sessions SET visible = 1, updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, id],
        )?;
        Ok(affected > 0)
    }

    // ── 消息 CRUD ───────────────────────────────────────────

    pub fn add_message(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
        done: bool,
        reasoning: Option<&str>,
        tool_calls: Option<&str>,
    ) -> Result<MessageRow> {
        let conn = self.conn.lock().unwrap();
        let now = ts_now();
        conn.execute(
            "INSERT INTO messages (session_id, role, content, done, reasoning, reasoning_done, tool_calls, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7)",
            rusqlite::params![session_id, role, content, if done { 1 } else { 0 }, reasoning, tool_calls, now],
        )?;
        // 更新会话时间
        conn.execute(
            "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, session_id],
        )?;
        let id = conn.last_insert_rowid();
        Ok(MessageRow {
            id,
            session_id: session_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            done,
            reasoning: reasoning.map(|s| s.to_string()),
            reasoning_done: false,
            tool_calls: tool_calls.map(|s| s.to_string()),
            created_at: now,
        })
    }

    /// 更新消息字段。fields_json 是 JSON 对象，包含要更新的字段：
    /// content, done, reasoning, reasoning_done, tool_calls
    pub fn update_message(&self, id: i64, fields_json: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let fields: serde_json::Value =
            serde_json::from_str(fields_json).with_context(|| "解析更新字段 JSON 失败")?;

        let mut set_parts: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(v) = fields.get("content").and_then(|v| v.as_str()) {
            set_parts.push("content = ?".into());
            params.push(Box::new(v.to_string()));
        }
        if let Some(v) = fields.get("done").and_then(|v| v.as_bool()) {
            set_parts.push("done = ?".into());
            params.push(Box::new(if v { 1 } else { 0 }));
        }
        if let Some(v) = fields.get("reasoning").and_then(|v| v.as_str()) {
            set_parts.push("reasoning = ?".into());
            params.push(Box::new(v.to_string()));
        }
        if let Some(v) = fields.get("reasoning_done").and_then(|v| v.as_bool()) {
            set_parts.push("reasoning_done = ?".into());
            params.push(Box::new(if v { 1 } else { 0 }));
        }
        if fields.get("tool_calls").is_some() {
            let tc_str = serde_json::to_string(&fields["tool_calls"]).unwrap_or_default();
            set_parts.push("tool_calls = ?".into());
            params.push(Box::new(tc_str));
        }

        if set_parts.is_empty() {
            return Ok(false);
        }

        let sql = format!("UPDATE messages SET {} WHERE id = ?", set_parts.join(", "));
        let mut stmt = conn.prepare(&sql)?;

        // rusqlite 的 params_from_iter 可以从 &[&dyn ToSql] 工作
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let affected = stmt.execute(rusqlite::params_from_iter(param_refs.iter().copied().chain(std::iter::once(&id as &dyn rusqlite::types::ToSql))))?;

        Ok(affected > 0)
    }

    pub fn get_messages(&self, session_id: &str) -> Result<Vec<MessageRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, role, content, done, reasoning, reasoning_done, tool_calls, created_at \
             FROM messages WHERE session_id = ?1 ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(rusqlite::params![session_id], |row| {
            let done_int: i32 = row.get(4)?;
            let rd_int: i32 = row.get(6)?;
            Ok(MessageRow {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                done: done_int != 0,
                reasoning: row.get(5)?,
                reasoning_done: rd_int != 0,
                tool_calls: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn clear_messages(&self, session_id: &str) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(
            "DELETE FROM messages WHERE session_id = ?1",
            rusqlite::params![session_id],
        )?;
        Ok(affected as u64)
    }

    // ── 配置读写 ────────────────────────────────────────────

    pub fn get_config(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT v FROM config WHERE k = ?1")?;
        let mut rows = stmt.query_map(rusqlite::params![key], |row| row.get(0))?;
        Ok(rows.next().transpose()?)
    }

    pub fn set_config(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO config (k, v) VALUES (?1, ?2) ON CONFLICT(k) DO UPDATE SET v = excluded.v",
            rusqlite::params![key, value],
        )?;
        Ok(())
    }

    pub fn delete_config(&self, key: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM config WHERE k = ?1", rusqlite::params![key])?;
        Ok(affected > 0)
    }

    // ── 迁移：从 localStorage JSON 批量导入 ──────────────────

    /// 将前端 localStorage 的完整 JSON 导入 SQLite。
    /// frontend_projects_json 格式：{ projects: [...], activeProjectId, activeConversationId }
    /// 返回导入的消息总数。
    pub fn migrate_from_localstorage(&self, json: &str) -> Result<u32> {
        let conn = self.conn.lock().unwrap();
        let data: serde_json::Value =
            serde_json::from_str(json).with_context(|| "解析迁移数据 JSON 失败")?;

        let projects = data
            .get("projects")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("迁移数据中缺少 projects 数组"))?;

        let mut total_messages: u32 = 0;

        for project in projects {
            let project_id = project.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let project_name = project.get("name").and_then(|v| v.as_str()).unwrap_or("未命名项目");
            let project_source = project.get("source").and_then(|v| v.as_str()).unwrap_or("local");
            let project_path = project.get("path").and_then(|v| v.as_str()).map(|s| s.to_string());
            let created_at = project.get("createdAt").and_then(|v| v.as_i64()).unwrap_or(0);
            let updated_at = project.get("updatedAt").and_then(|v| v.as_i64()).unwrap_or(0);

            conn.execute(
                "INSERT OR IGNORE INTO projects (id, name, path, source, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![project_id, project_name, project_path, project_source, created_at, updated_at],
            )?;

            if let Some(convs) = project.get("conversations").and_then(|v| v.as_array()) {
                for conv in convs {
                    let conv_id = conv.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let conv_title = conv.get("title").and_then(|v| v.as_str()).unwrap_or("未命名会话");
                    let title_source = conv.get("titleSource").and_then(|v| v.as_str()).unwrap_or("default");
                    let visible = conv.get("visible").and_then(|v| v.as_bool()).unwrap_or(true);
                    let vis_int: i32 = if visible { 1 } else { 0 };
                    let conv_created = conv.get("createdAt").and_then(|v| v.as_i64()).unwrap_or(0);
                    let conv_updated = conv.get("updatedAt").and_then(|v| v.as_i64()).unwrap_or(0);

                    conn.execute(
                        "INSERT OR IGNORE INTO sessions (id, project_id, title, title_source, visible, created_at, updated_at) \
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        rusqlite::params![conv_id, project_id, conv_title, title_source, vis_int, conv_created, conv_updated],
                    )?;

                    if let Some(msgs) = conv.get("messages").and_then(|v| v.as_array()) {
                        for msg in msgs {
                            let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("");
                            let content = msg.get("content").and_then(|v| v.as_str()).unwrap_or("");
                            let done = msg.get("done").and_then(|v| v.as_bool()).unwrap_or(false);
                            let done_int: i32 = if done { 1 } else { 0 };
                            let reasoning = msg.get("reasoning").and_then(|v| v.as_str()).map(|s| s.to_string());
                            let reasoning_done = msg.get("reasoningDone").and_then(|v| v.as_bool()).unwrap_or(false);
                            let rd_int: i32 = if reasoning_done { 1 } else { 0 };
                            let tool_calls = msg.get("toolCalls").map(|v| v.to_string());
                            let msg_ts = msg.get("createdAt").and_then(|v| v.as_i64()).unwrap_or_else(ts_now);

                            conn.execute(
                                "INSERT INTO messages (session_id, role, content, done, reasoning, reasoning_done, tool_calls, created_at) \
                                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                                rusqlite::params![conv_id, role, content, done_int, reasoning, rd_int, tool_calls, msg_ts],
                            )?;
                            total_messages += 1;
                        }
                    }
                }
            }
        }

        // 标记迁移完成
        conn.execute(
            "INSERT INTO config (k, v) VALUES ('migrated', 'true') ON CONFLICT(k) DO UPDATE SET v = 'true'",
            [],
        )?;

        Ok(total_messages)
    }

    /// 查询是否已从 localStorage 迁移过。
    pub fn is_migrated(&self) -> Result<bool> {
        self.get_config("migrated").map(|v| v.as_deref() == Some("true"))
    }

    // ── 兼容旧版 API（保留原有方法签名）──────────────────────

    /// 旧版 create_session（无 project_id 参数），用于兼容已有调用方。
    /// 新代码请使用完整签名的 create_session。
    #[doc(hidden)]
    pub fn create_session_legacy(&self, id: &str, title: &str) -> Result<Session> {
        let conn = self.conn.lock().unwrap();
        let now = ts_now();
        let now_str = ts_to_iso(now);
        // 尝试旧表结构（sessions 仅 id/title/created_at/updated_at）
        conn.execute(
            "INSERT INTO sessions (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, title, now, now],
        )?;
        Ok(Session {
            id: id.to_string(),
            title: title.to_string(),
            created_at: now_str.clone(),
            updated_at: now_str,
        })
    }

    /// 旧版 list_sessions，兼容旧调用。
    #[doc(hidden)]
    pub fn list_sessions_legacy(&self) -> Result<Vec<Session>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, created_at, updated_at FROM sessions ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let ca: i64 = row.get(2)?;
            let ua: i64 = row.get(3)?;
            Ok(Session {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: ts_to_iso(ca),
                updated_at: ts_to_iso(ua),
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// 旧版 get_session，兼容旧调用。
    #[doc(hidden)]
    pub fn get_session_legacy(&self, id: &str) -> Result<Option<Session>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, created_at, updated_at FROM sessions WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![id], |row| {
            let ca: i64 = row.get(2)?;
            let ua: i64 = row.get(3)?;
            Ok(Session {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: ts_to_iso(ca),
                updated_at: ts_to_iso(ua),
            })
        })?;
        Ok(rows.next().transpose()?)
    }

    /// 旧版 delete_session，兼容旧调用。
    #[doc(hidden)]
    pub fn delete_session_legacy(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM sessions WHERE id = ?1", rusqlite::params![id])?;
        Ok(())
    }

    /// 旧版 update_session_title，兼容旧调用。
    #[doc(hidden)]
    pub fn update_session_title_legacy(&self, id: &str, title: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = ts_now();
        conn.execute(
            "UPDATE sessions SET title = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![title, now, id],
        )?;
        Ok(())
    }

    /// 旧版 touch_session，兼容旧调用。
    #[doc(hidden)]
    pub fn touch_session_legacy(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = ts_now();
        conn.execute(
            "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, id],
        )?;
        Ok(())
    }

    /// 旧版 add_message，兼容旧调用。
    #[doc(hidden)]
    pub fn add_message_legacy(&self, session_id: &str, role: &str, content: &str) -> Result<Message> {
        let conn = self.conn.lock().unwrap();
        let now = ts_now();
        conn.execute(
            "INSERT INTO messages (session_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![session_id, role, content, now],
        )?;
        conn.execute(
            "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, session_id],
        )?;
        let id = conn.last_insert_rowid();
        Ok(Message {
            id,
            session_id: session_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            created_at: ts_to_iso(now),
        })
    }

    /// 旧版 get_messages，兼容旧调用。
    #[doc(hidden)]
    pub fn get_messages_legacy(&self, session_id: &str) -> Result<Vec<Message>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, role, content, created_at FROM messages WHERE session_id = ?1 ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(rusqlite::params![session_id], |row| {
            let ca: i64 = row.get(4)?;
            Ok(Message {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                created_at: ts_to_iso(ca),
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// 旧版 clear_messages，兼容旧调用。
    #[doc(hidden)]
    pub fn clear_messages_legacy(&self, session_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM messages WHERE session_id = ?1",
            rusqlite::params![session_id],
        )?;
        Ok(())
    }
}

// ── 工具函数 ─────────────────────────────────────────────────

/// 当前毫秒级时间戳。
pub fn ts_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// 毫秒时间戳 → ISO 8601 字符串（给旧版 API 用）。
fn ts_to_iso(ts: i64) -> String {
    let secs = ts / 1000;
    let millis = ts % 1000;
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;
    let (year, month, day) = days_to_date(days_since_epoch);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        year, month, day, hours, minutes, seconds, millis
    )
}

fn days_to_date(days: i64) -> (i64, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

fn default_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .map(|p| PathBuf::from(p).join("xuflow").join("xuflow.db"))
            .unwrap_or_else(|_| PathBuf::from("xuflow.db"))
    }
    #[cfg(target_os = "macos")]
    {
        home_dir()
            .map(|p| p.join("Library").join("Application Support").join("xuflow").join("xuflow.db"))
            .unwrap_or_else(|| PathBuf::from("xuflow.db"))
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var("XDG_DATA_HOME")
            .map(|p| PathBuf::from(p).join("xuflow").join("xuflow.db"))
            .or_else(|_| {
                home_dir().map(|p| p.join(".local").join("share").join("xuflow").join("xuflow.db"))
            })
            .unwrap_or_else(|_| PathBuf::from("xuflow.db"))
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}
