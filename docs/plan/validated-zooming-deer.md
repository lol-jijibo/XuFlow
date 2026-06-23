# 修正 SQLite 数据库路径 & 在设置页显示路径

## Context

当前 [lib.rs:25](desktop/src-tauri/src/lib.rs#L25) 硬编码了测试路径：
```rust
let db_path = std::path::PathBuf::from(r"D:\Projects-star\XuFlow-sqlite_content\xuflow.db");
```

而 [session.rs:723-745](packages/core/src/memory/session.rs#L723-L745) 已有完善的跨平台 `default_data_dir()`，正确路径应为：
- Windows: `%APPDATA%/xuflow/xuflow.db`
- macOS: `~/Library/Application Support/xuflow/xuflow.db`
- Linux: `~/.local/share/xuflow/xuflow.db`

同时，设置页当前硬编码显示 `%APPDATA%/xuflow/xuflow.db`，用户无法确认实际路径。

## 变更计划

### 1. `packages/core/src/memory/session.rs` — 存储路径并暴露 getter

- `SessionStore` 结构体新增 `path: PathBuf` 字段
- `open()` 中将解析后的实际路径存入 `self.path`
- 新增 `pub fn db_path(&self) -> String` 返回路径字符串

### 2. `desktop/src-tauri/src/lib.rs` — 移除硬编码路径

- 删除 `let db_path = ...` 行
- 改为 `let store = SessionStore::open(None)` 使用系统默认路径

### 3. `desktop/src-tauri/src/commands/persistence.rs` — 新增 `db_get_path` 命令

- 遵循现有命令模式，新增：
```rust
#[tauri::command]
pub async fn db_get_path(state: State<'_, Arc<DbState>>) -> Result<String, String> {
    Ok(state.store.db_path())
}
```

### 4. `desktop/src-tauri/src/lib.rs` — 注册新命令

- 在 `generate_handler!` 中添加 `commands::persistence::db_get_path`

### 5. `desktop/src/components/config/SettingsPanel.vue` — 动态显示路径

- 新增 `dbPath` ref
- `onMounted` 中调用 `invoke<string>("db_get_path")` 获取实际路径
- 将 "数据文件位置" 下的硬编码文本替换为动态路径显示
- 可选：添加点击复制路径或打开目录的功能

## 关键文件

| 文件 | 修改内容 |
|---|---|
| `packages/core/src/memory/session.rs` | `SessionStore` 加 `path` 字段 + `db_path()` getter |
| `desktop/src-tauri/src/lib.rs` | 改为 `SessionStore::open(None)` + 注册新命令 |
| `desktop/src-tauri/src/commands/persistence.rs` | 新增 `db_get_path` 命令 |
| `desktop/src/components/config/SettingsPanel.vue` | 动态获取并显示数据库路径 |

## 验证

1. `cargo build` 编译通过
2. 启动桌面端，打开设置 → 数据库，确认显示正确的系统路径（如 `C:\Users\Lenovo\AppData\Roaming\xuflow\xuflow.db`）
3. 发送一条测试消息，关闭重开后确认会话内容保留
4. 将旧的 `D:\Projects-star\XuFlow-sqlite_content\xuflow.db` 手动复制到新路径可恢复之前的数据
