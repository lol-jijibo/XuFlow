# 持久化层：MySQL → SQLite 迁移

## 背景

当前桌面端使用 MySQL（`sqlx` + `mysql` feature），要求用户自行安装 MySQL 服务、建库、配置连接。这对桌面应用来说是反模式——同类产品（Codex、Claude Code、Cursor）全部使用本地文件存储，零配置开箱即用。

核心发现：`packages/core/` 中已有一个基于 `rusqlite` 的 `SessionStore`，schema 和 CRUD 方法已存在，只是字段不完整、未被桌面端使用。改造它是成本最低的路径。

## 目标

- 删除 MySQL 依赖，桌面端默认使用 SQLite 文件存储
- 用户零配置，打开应用即可用
- 保持前端 invoke 接口兼容（command 名称不变）

## 改造步骤

### 第一步：扩展 `packages/core/src/memory/session.rs` 的 SessionStore

当前 SessionStore 只有 `sessions` + `messages` 两张表，缺少：
- `projects` 表（含 path、source 字段）
- `config` 表（key-value）
- messages 表缺少 `done`、`reasoning`、`reasoning_done`、`tool_calls` 字段
- sessions 表缺少 `title_source`、`visible`、`project_id` 字段
- 时间戳用字符串，前端用毫秒整数

需要新增/修改：

```sql
-- 项目表（新增）
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT,
    source TEXT NOT NULL DEFAULT 'local',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- 会话表（改造：加 project_id FK、title_source、visible）
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

-- 消息表（改造：加 done、reasoning、reasoning_done、tool_calls）
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

-- 配置表（新增）
CREATE TABLE IF NOT EXISTS config (
    k TEXT PRIMARY KEY,
    v TEXT NOT NULL
);
```

新增方法：
- 项目 CRUD：`create_project`, `list_projects`, `update_project_name`, `delete_project`
- 会话扩展：`create_session` 加 project_id/title_source/visible 参数；`list_sessions_by_project`；`reveal_session`
- 消息扩展：`add_message` 加 reasoning/tool_calls 字段；`update_message` 用于流式更新
- 配置读写：`get_config`, `set_config`, `delete_config`
- 迁移：`migrate_from_localstorage`（从 JSON 批量导入）

### 第二步：替换 `desktop/src-tauri/src/commands/persistence.rs`

**完全重写**。删除所有 MySQL/sqlx 代码，改为：

```rust
// 核心结构：持有一个 SessionStore，应用启动即自动打开
pub struct DbState {
    pub store: SessionStore,
}
```

每个 Tauri 命令从 `State<'_, Arc<DbState>>` 取出 `store`，直接调用对应方法。由于 `SessionStore` 内部用 `Mutex<Connection>` 是同步的，Tauri 命令需要用 `tokio::task::spawn_blocking` 包装，避免阻塞异步运行时。（或者改为 `tokio::sync::Mutex` + 在 blocking 线程池中执行）

**命令名保持不变**（前端无需改 invoke 名称）：
- `db_connect` → 改为打开/创建 SQLite 文件（或直接删除，因为启动即连接）
- `db_is_connected` → 始终返回 true
- 其余 CRUD 命令保留，内部委托给 SessionStore

### 第三步：更新 `desktop/src-tauri/src/lib.rs`

- 删除 MySQL 相关的 `DbState::new()` 初始化，改为 `DbState::new(app_data_dir)`
- 删除 `db_test_connection`、`db_disconnect` 的命令注册（前端不再需要）
- 使用 Tauri 的 `app.path().app_data_dir()` 获取数据库路径

### 第四步：清理前端

**`desktop/src/stores/project.ts`：**
- 初始化时直接调用 `tryLoadFromMySql()` —— 因为 SQLite 始终"已连接"，数据从 SQLite 加载
- 去掉 `dbConnected` 的条件判断，持久化始终走 SQLite
- 实际上可以去掉 localStorage 回退，因为 SQLite 是本地文件，不会像 MySQL 那样连不上

**`desktop/src/stores/config.ts`：**
- 同上，`loadFromMySql` 变成主要的配置加载路径
- 去掉 `dbConnected` 条件

**`desktop/src/components/config/SettingsPanel.vue`：**
- "数据库"设置区块改为简单的状态显示（"SQLite 存储路径: xxx"）
- 或直接删除该区块，因为无需配置
- 保留 localStorage → SQLite 迁移功能（一次性）

### 第五步：清理依赖

**`desktop/src-tauri/Cargo.toml`：**
- 删除 `sqlx` 依赖行
- 删除 `tauri-plugin-sql`（本来就没用）
- 确认 `rusqlite` 已在 workspace 或 core 依赖中

**`Cargo.toml`（workspace）：**  
- 删除 `sqlx` 的 workspace 定义（如果无其他地方使用）

## 涉及文件

| 文件 | 操作 |
|------|------|
| `packages/core/src/memory/session.rs` | **扩展** — 加 projects/config 表、补齐字段、加 CRUD 方法 |
| `desktop/src-tauri/src/commands/persistence.rs` | **重写** — sqlx+mysql → SessionStore 封装 |
| `desktop/src-tauri/src/lib.rs` | **修改** — 初始化逻辑、命令注册 |
| `desktop/src-tauri/Cargo.toml` | **修改** — 删 sqlx、tauri-plugin-sql |
| `desktop/src/stores/project.ts` | **修改** — 去掉 dbConnected 条件、始终走 SQLite |
| `desktop/src/stores/config.ts` | **修改** — 同上 |
| `desktop/src/components/config/SettingsPanel.vue` | **修改** — 数据库区块改为状态展示 |

## 验证方式

1. `cargo build -p xuflow-desktop` 编译通过
2. 启动应用，检查 `%APPDATA%/xuflow/xuflow.db` 是否自动创建
3. 创建项目 → 会话 → 发送消息 → 重启应用 → 数据完整保留
4. 检查旧的 localStorage 数据是否被迁移到 SQLite
5. 删除 `sqlx` 和 `tauri-plugin-sql` 依赖后编译通过
