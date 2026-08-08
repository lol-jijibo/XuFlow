# 多会话并行支持 — 实现方案

## Context

当前 Xuflow 桌面端后端只有一个全局 `AgentSession`（含单个 `Mutex<AgentLoop>`），导致：
- 同一时间只能运行一个对话
- 切换会话时内存中的消息上下文丢失
- 前端虽已支持多会话 UI 和 SQLite 持久化，但后端不感知 sessionId

目标：将单体 `AgentSession` 改造为按 sessionId 索引的会话池，支持多会话上下文保持和并行执行。

## 核心思路

```
改造前: Arc<AgentSession>  { agent: Mutex<AgentLoop> }      // 1个
改造后: Arc<SessionManager> { sessions: DashMap<String, Handle> }  // N个
```

每个 `Handle` 持有独立的 `AgentLoop`、`cancelled` 标志、`approval_tx`。MCP 管理器全局共享。

## 事件路由策略

Tauri 事件全局广播。并行场景下，后端在所有事件 payload 中附带 `sessionId`。前端 `useTauriEvent.ts` 读取 payload 中的 `sessionId` 与 `activeConversationId` 比对，只处理当前活跃会话的事件，丢弃其他会话的事件。

## 修改文件清单

### 1. `packages/core/src/agent/loop_.rs` — 新增 `with_messages()` 构造器

在 `AgentLoop` 上新增方法，支持从外部注入完整消息历史（恢复会话时使用）：

```rust
/// 批量注入已有消息历史（从 SQLite 恢复会话上下文时使用）。
/// 会替换当前 messages，后续 run() 在此基础上追加。
pub fn with_messages(mut self, messages: Vec<ChatMessage>) -> Self {
    self.messages = messages;
    self
}
```

`ChatMessage` 需额外派生 `Clone`（已确认可行）。

### 2. `desktop/src-tauri/src/commands/chat.rs` — 核心重构

#### 2a. 新增 `SessionHandle` 结构体

```rust
struct SessionHandle {
    agent: Mutex<AgentLoop>,
    cancelled: Arc<AtomicBool>,
    approval_tx: ApprovalChannel,
    /// 当前会话后端实例（切换模型时需要重建）
    backend: Arc<dyn LlmBackend>,
    working_dir: String,
}
```

#### 2b. 新增 `SessionManager` 结构体（替代 `AgentSession`）

```rust
pub struct SessionManager {
    /// 活跃会话池：sessionId → Handle
    sessions: DashMap<String, Arc<SessionHandle>>,
    /// 全局 MCP 管理器（所有会话共享连接）
    mcp_manager: Arc<Mutex<Option<Arc<McpManager>>>>,
    /// MCP 初始化错误
    mcp_init_errors: Arc<Mutex<Vec<String>>>,
    /// 当前模型/Provider/API Key（用于新建会话时构造后端）
    current_provider: Mutex<String>,
    current_model: Mutex<String>,
    current_api_key: Mutex<String>,
    app_handle: tauri::AppHandle,
    /// 持久化层引用（恢复消息历史用）
    db_state: Option<Arc<crate::commands::persistence::DbState>>,
}
```

使用 `DashMap`（无需全局锁的高并发 HashMap）管理会话池。

#### 2c. `SessionManager` 核心方法

- `get_or_create(session_id)`: 查找或创建会话的 Handle
- `restore_session(session_id)`: 从 SQLite 加载消息历史，重建 AgentLoop 上下文
- `remove_session(session_id)`: 关闭并清理会话
- `reconfigure(provider, model, api_key)`: 更新全局配置（新建会话使用新配置）

#### 2d. `build_agent` 改为关联函数

从 `AgentSession` 的方法改为 `SessionManager` 的关联函数，接收 `mcp_manager` 引用和 `cancelled` 参数。

#### 2e. 改造已有 Tauri 命令

**`send_message`** — 新增 `session_id: String` 参数：
```rust
pub async fn send_message(
    content: String,
    session_id: String,   // ← 新增
    state: tauri::State<'_, Arc<SessionManager>>,
    app: tauri::AppHandle,
) -> Result<String, String>
```
- 通过 `state.get_or_create(&session_id)` 获取会话 Handle
- 锁定该会话的 `agent`，调用 `run()`
- 事件转发器中所有 `emit` 的 payload 附带 `sessionId` 字段

**`stop_generation`** — 新增 `session_id: String` 参数，只取消指定会话。

**`respond_approval`** — 新增 `session_id: String` 参数，路由到正确会话的 approval channel。

**`configure_agent`** — 保存配置到 `SessionManager`，同时重建所有已有会话的后端（或仅影响新会话，权衡后选择仅影响新会话，减少复杂度）。

**`generate_title`** — 新增 `session_id: String` 参数（或保留使用当前活跃会话的后端）。

**`set_context_window` / `set_min_user_turns`** — 新增 `session_id: String` 参数。

#### 2f. 新增 Tauri 命令

**`restore_session`**：
```rust
#[tauri::command]
pub async fn restore_session(
    session_id: String,
    state: tauri::State<'_, Arc<SessionManager>>,
    db: tauri::State<'_, Arc<DbState>>,
) -> Result<(), String>
```
从 SQLite 加载 session 的消息历史，通过 `with_messages()` 注入 AgentLoop。

**`close_session`**：
```rust
#[tauri::command]
pub async fn close_session(
    session_id: String,
    state: tauri::State<'_, Arc<SessionManager>>,
) -> Result<(), String>
```
清理指定会话的 Handle，释放内存。

### 3. `desktop/src-tauri/src/lib.rs` — 注册新状态和命令

- `app.manage(Arc::new(SessionManager::new(...)))` 替代 `Arc::new(AgentSession::new(...))`
- 注册新增的 Tauri 命令：`restore_session`, `close_session`
- `send_message`, `stop_generation`, `respond_approval` 等已存在，无需改注册（签名变更自动反映）

### 4. `desktop/src/stores/agent.ts` — 前端适配

```typescript
// sendMessage — 传入当前活跃的 conversationId
async function sendMessage(content: string) {
  const convId = useProjectStore().activeConversationId;
  // ...
  await invoke("send_message", { content, sessionId: convId });
}

// stopGeneration — 传入 sessionId
async function stopGeneration() {
  const convId = useProjectStore().activeConversationId;
  await invoke("stop_generation", { sessionId: convId });
  // ...
}

// 新增：切换会话时恢复后端上下文
async function restoreSession(sessionId: string) {
  await invoke("restore_session", { sessionId });
}
```

### 5. `desktop/src/composables/useTauriEvent.ts` — 事件过滤

所有事件处理器开头增加活跃会话判断：

```typescript
// 示例：agent:text-delta
await listen<string>("agent:text-delta", (event) => {
  const payload = JSON.parse(event.payload);
  // 只处理当前活跃会话的事件
  if (payload.sessionId !== projectStore.activeConversationId) return;
  const msg = lastStreamingMsg();
  if (!msg) return;
  msg.content += payload.delta;
  schedulePersist();
});
```

同理修改所有 13 个事件监听器。

### 6. `desktop/src/components/chat/ChatPanel.vue` — 会话切换时恢复

在 `watch(activeConversationId)` 中调用 `restoreSession()`：

```typescript
watch(
  () => projectStore.activeConversationId,
  async (newId) => {
    if (newId) {
      await agentStore.restoreSession(newId);
    }
    // ... 现有的滚动位置逻辑
  }
);
```

### 7. `desktop/src/components/layout/Sidebar.vue` — 切换会话时调用 restore

侧边栏点击切换会话时，除了更新 `activeConversationId` 外，无需额外改动（watch 已覆盖）。

## 并发控制

- 使用 `DashMap` 管理会话池，读无锁、写有锁但粒度到单个 entry
- 每个会话的 `AgentLoop::run()` 独立执行（各自的 `tokio::spawn` 任务）
- 前端 `isRunning` 变为按会话计算：`isRunning(sessionId)` 或维护一个 `runningSessions: Set<string>`
- 不做硬性并发上限（用户通常不会同时跑超过 2-3 个会话）

## 边界情况

1. **会话被删除时正在运行**：`close_session` 先设置 cancelled flag，等 run() 自然退出后再移除
2. **应用退出**：`SessionManager` 的 Drop 实现遍历所有会话，设置 cancelled 并等待任务完成
3. **MCP 连接**：全局共享，首次 `configure_agent` 时初始化，后续不重建
4. **模型切换**：`configure_agent` 更新全局配置，新会话用新配置；已有会话保持原后端不变（避免中断运行中的对话）

## 验证方案

1. 启动桌面端，创建两个会话 A 和 B
2. 在 A 中发送消息，等待回复 → 确认正常流式输出
3. 切换到 B，发送消息 → 确认 B 独立运行，不受 A 影响
4. 切回 A → 确认 A 的上下文（消息历史）完整保留
5. 在 A 运行期间切到 B 并发送 → 确认两个会话真正并行（通过日志时间戳验证）
6. 在 A 运行期间点停止 → 确认只取消 A，B 不受影响
7. 删除会话 → 确认对应的 Handle 被清理
