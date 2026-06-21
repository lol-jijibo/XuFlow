# 数据持久化重构：localStorage → MySQL

## Context

当前 Xuflow 桌面端将所有数据（项目、会话、消息、配置）存储在前端 `localStorage` 中，物理路径在：
```
C:\Users\Lenovo\AppData\Local\com.xuflow.desktop\EBWebView\Default\Local Storage\leveldb\
```

**问题：**
1. 数据存在 C 盘，不可配置路径
2. `localStorage` 有 5-10MB 容量限制，对话增多后可能溢出
3. 数据以单 JSON blob 序列化，无法增量读写，大规模时性能差
4. 无法直接用数据库工具（Navicat/DBeaver 等）浏览/查询数据

**目标：**
- 使用 MySQL 数据库替代 `localStorage`
- MySQL 连接信息可在设置页面配置（host / port / user / password / database）
- 保留前端 Pinia store 的响应式编程模型，平滑迁移
- 首次启动自动迁移 `localStorage` 旧数据到 MySQL

---

## 技术选型

使用 `sqlx` crate（已存在于依赖树中，`tauri-plugin-sql` 的传递依赖），直接通过 Rust Tauri 命令访问 MySQL。

- **Crate:** `sqlx = { version = "0.8", features = ["runtime-tokio", "mysql"] }`
- **连接池:** `sqlx::MySqlPool`（内置连接池）
- **异步:** 与项目已有的 tokio v1 完全兼容
- **依赖:** 仅添加到 `desktop/src-tauri/Cargo.toml`，不污染 `packages/core`

## 数据流架构

```
┌──────────────────────────────────────┐
│  前端 (Vue/Pinia)                     │
│  project.ts / config.ts               │
│       ↓ invoke()                      │
├──────────────────────────────────────┤
│  Rust Tauri 命令层                     │
│  commands/persistence.rs (新建)        │
│  ┌────────────────────────────────┐   │
│  │  MySqlPool (sqlx)              │   │
│  │  - 项目 CRUD                    │   │
│  │  - 会话 CRUD                    │   │
│  │  - 消息 CRUD                    │   │
│  │  - 配置读写                     │   │
│  │  - 连接测试                     │   │
│  │  - 数据迁移(localStorage→MySQL) │   │
│  └────────────────────────────────┘   │
├──────────────────────────────────────┤
│  MySQL 服务器                          │
│  ┌────────────────────────────────┐   │
│  │  xuflow 数据库                  │   │
│  │  - projects                    │   │
│  │  - sessions                    │   │
│  │  - messages                    │   │
│  │  - config                      │   │
│  └────────────────────────────────┘   │
└──────────────────────────────────────┘
```

---

## 实现计划

### 阶段一：添加 MySQL 依赖

**文件：** `desktop/src-tauri/Cargo.toml`

```toml
# 新增
sqlx = { version = "0.8", features = ["runtime-tokio", "mysql"] }
```

**文件：** `desktop/src-tauri/Cargo.toml`（修改已有行）

```toml
# tauri-plugin-sql 不需要加 mysql feature，因为我们直接走 sqlx
# 保持不变即可
tauri-plugin-sql = { version = "2", features = ["sqlite"] }
```

### 阶段二：创建 MySQL 持久化模块

**文件：** `desktop/src-tauri/src/commands/persistence.rs`（新建）

#### 数据库初始化

```rust
use sqlx::mysql::MySqlPool;
use sqlx::Row;

// 创建连接池
async fn create_pool(opts: &MySqlOpts) -> Result<MySqlPool, sqlx::Error> {
    let url = format!(
        "mysql://{}:{}@{}:{}/{}",
        opts.user, opts.password, opts.host, opts.port, opts.database
    );
    MySqlPool::connect(&url).await
}

// 建表（首次启动自动执行）
async fn init_schema(pool: &MySqlPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS projects (
            id VARCHAR(64) PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            path VARCHAR(1024),
            source VARCHAR(32) NOT NULL DEFAULT 'local',
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4"
    ).execute(pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS sessions (
            id VARCHAR(64) PRIMARY KEY,
            project_id VARCHAR(64) NOT NULL,
            title VARCHAR(255) NOT NULL,
            title_source VARCHAR(16) NOT NULL DEFAULT 'default',
            visible TINYINT NOT NULL DEFAULT 1,
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL,
            FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4"
    ).execute(pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS messages (
            id BIGINT AUTO_INCREMENT PRIMARY KEY,
            session_id VARCHAR(64) NOT NULL,
            role VARCHAR(16) NOT NULL,
            content LONGTEXT NOT NULL,
            done TINYINT NOT NULL DEFAULT 0,
            reasoning LONGTEXT,
            reasoning_done TINYINT DEFAULT 0,
            tool_calls LONGTEXT,
            created_at BIGINT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
            INDEX idx_session_id (session_id)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4"
    ).execute(pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS config (
            k VARCHAR(128) PRIMARY KEY,
            v LONGTEXT NOT NULL
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4"
    ).execute(pool).await?;

    Ok(())
}
```

#### MySQL 连接配置结构体

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MySqlOpts {
    pub host: String,       // 默认 "127.0.0.1"
    pub port: u16,          // 默认 3306
    pub user: String,       // 默认 "root"
    pub password: String,
    pub database: String,   // 默认 "xuflow"
}
```

#### Tauri 命令列表

| 命令 | 参数 | 返回 | 说明 |
|---|---|---|---|
| `db_connect` | `MySqlOpts` | `bool` | 测试并建立连接，成功后初始化 schema |
| `db_test_connection` | `MySqlOpts` | `bool` | 仅测试连接，不持久化 |
| `db_create_project` | `name, source` | `Project` | 新建项目 |
| `db_list_projects` | — | `Vec<Project>` | 列出所有项目 |
| `db_update_project_name` | `id, name` | `bool` | 重命名项目 |
| `db_delete_project` | `id` | `bool` | 删除项目（级联删除会话+消息） |
| `db_create_session` | `project_id, title, title_source, visible` | `Session` | 新建会话 |
| `db_list_sessions` | `project_id` | `Vec<Session>` | 列出项目下所有会话 |
| `db_update_session_title` | `id, title` | `bool` | 重命名会话 |
| `db_delete_session` | `id` | `bool` | 删除会话（级联删除消息） |
| `db_add_message` | `session_id, role, content_json` | `Message` | 新增消息 |
| `db_get_messages` | `session_id` | `Vec<Message>` | 获取会话消息 |
| `db_update_message` | `id, content_json` | `bool` | 更新消息（流式 delta 或 完成标记） |
| `db_get_config` | `key` | `Option<String>` | 读取配置 |
| `db_set_config` | `key, value` | `bool` | 写入配置 |
| `db_migrate_from_localstorage` | `projects_json` | `u32` | 迁移旧 localStorage 数据，返回迁移条数 |

#### 应用状态管理

```rust
// 在 lib.rs 的 setup() 中注册
pub struct DbState {
    pub pool: Arc<tokio::sync::RwLock<Option<MySqlPool>>>,
    pub connected: AtomicBool,
}
```

### 阶段三：注册命令和状态

**文件：** `desktop/src-tauri/src/lib.rs`

```rust
// setup() 中新增
let db_state = DbState {
    pool: Arc::new(tokio::sync::RwLock::new(None)),
    connected: AtomicBool::new(false),
};
app.manage(Arc::new(db_state));
```

**文件：** `desktop/src-tauri/src/commands/mod.rs`

```rust
pub mod persistence;
```

**invoke_handler 新增：**
```rust
commands::persistence::db_connect,
commands::persistence::db_test_connection,
commands::persistence::db_create_project,
commands::persistence::db_list_projects,
// ... 其余所有命令
```

### 阶段四：前端 Pinia store 改造

**文件：** `desktop/src/stores/project.ts`

核心改动：将 `localStorage` 调用替换为 `invoke()` 调用。

```typescript
// 旧：loadState() 从 localStorage 读取
// 新：invoke("db_list_projects") + invoke("db_list_sessions") 从 MySQL 加载

// 旧：createProject(...) 操作 JS 数组 + localStorage.setItem
// 新：invoke("db_create_project", { name, source }) 写入 MySQL

// 旧：updateConversationTitle(...) 操作 JS 对象 + localStorage.setItem
// 新：invoke("db_update_session_title", { id, title }) 写入 MySQL

// persist() 变为空操作（或移除调用），因为每次变更已实时写入 MySQL
```

保留 Pinia 的响应式状态（`projects`, `activeProjectId`, `activeConversationId`），但数据源改为从 MySQL 加载。

**文件：** `desktop/src/stores/config.ts`

```typescript
// 旧：localStorage.getItem("xuflow-config")
// 新：invoke("db_get_config", { key: "xuflow-config" })

// 旧：localStorage.setItem("xuflow-config", JSON.stringify(data))
// 新：invoke("db_set_config", { key: "xuflow-config", value: JSON.stringify(data) })
```

**文件：** `desktop/src/composables/useTauriEvent.ts`

消息流式持久化：每个 delta 更新 MySQL 中对应消息行（而不是写 localStorage）。

### 阶段五：设置页面新增数据库配置

**文件：** `desktop/src/components/config/SettingsPanel.vue`

新增 "数据库" 设置区块（`activeSection = "database"`）：

```html
<!-- 数据库连接配置 -->
<NForm>
  <NFormItem label="主机地址">
    <NInput v-model:value="dbHost" placeholder="127.0.0.1" />
  </NFormItem>
  <NFormItem label="端口">
    <NInputNumber v-model:value="dbPort" :min="1" :max="65535" />
  </NFormItem>
  <NFormItem label="用户名">
    <NInput v-model:value="dbUser" placeholder="root" />
  </NFormItem>
  <NFormItem label="密码">
    <NInput type="password" v-model:value="dbPassword" />
  </NFormItem>
  <NFormItem label="数据库名">
    <NInput v-model:value="dbName" placeholder="xuflow" />
  </NFormItem>
  <NButton @click="testConnection">测试连接</NButton>
  <NButton type="primary" @click="saveAndConnect">保存并连接</NButton>
</NForm>
```

连接状态指示器（绿色/红色圆点 + 文字）。

### 阶段六：配置持久化

MySQL 连接配置本身需要持久化 —— 但此时数据库还没连上。解决方案：
- 连接配置仍然存 `localStorage`（仅此一个 key：`xuflow-db-config`）
- 应用启动时先从 `localStorage` 读取连接配置 → 连接 MySQL → 加载数据
- 或者存到本地配置文件（通过 Tauri `fs` API）

### 阶段七：数据迁移

首次成功连接 MySQL 后：
1. 检查 `config` 表中是否有 `migrated: true` 标记
2. 如果没有，读取 `localStorage` 的 `xuflow-projects`
3. 遍历所有项目 → 会话 → 消息，逐条插入 MySQL
4. 完成后写入 `migrated: true` 标记
5. 不清除 `localStorage` 旧数据（安全回退）

---

## 修改文件清单

| 文件 | 改动 |
|---|---|
| `desktop/src-tauri/Cargo.toml` | 加 `sqlx` 依赖（MySQL feature） |
| `desktop/src-tauri/src/commands/persistence.rs` | **新建** — MySQL 连接池 + CRUD 命令 |
| `desktop/src-tauri/src/commands/mod.rs` | 加 `pub mod persistence` |
| `desktop/src-tauri/src/lib.rs` | 注册 `DbState`，注册新命令 |
| `desktop/src/stores/project.ts` | localStorage → `invoke("db_xxx")` |
| `desktop/src/stores/config.ts` | localStorage → `invoke("db_get/set_config")` |
| `desktop/src/composables/useTauriEvent.ts` | 消息持久化改用 invoke |
| `desktop/src/components/config/SettingsPanel.vue` | 新增"数据库"设置区块 |

---

## 验证步骤

1. 启动 MySQL 服务（本地或远程）
2. 创建空数据库：`CREATE DATABASE xuflow CHARACTER SET utf8mb4;`
3. 启动应用 → 进入设置 → 数据库 → 填写连接信息 → 点"测试连接" → 显示成功
4. 点"保存并连接" → 自动建表 → 状态指示灯变绿
5. 创建项目/会话/消息 → 用 Navicat 查看 `projects`/`sessions`/`messages` 表验证数据
6. 双击重命名项目/会话 → 验证数据库更新
7. 重启应用 → 自动重连 → 验证数据正确加载
8. 修改设置 → 验证 `config` 表持久化
9. 旧 localStorage 数据 → 验证自动迁移到 MySQL
