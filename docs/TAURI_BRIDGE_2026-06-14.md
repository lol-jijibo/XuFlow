# Tauri 桥接层实现文档

**日期**: 2026-06-14  
**涉及模块**: `packages/desktop/src-tauri/src/commands/chat.rs`, `packages/desktop/src/stores/agent.ts`, `packages/desktop/src/composables/useTauriEvent.ts`

---

## 一、架构概览

```
┌──────────────────────────────────────────────────┐
│  Frontend (Vue3 + Pinia)                         │
│                                                  │
│  ChatPanel ──> agentStore.sendMessage()          │
│                  │ invoke("send_message")         │
│                  ▼                                │
│  useTauriEvent <── listen("agent:text-delta")    │
│       │           listen("agent:tool-call")       │
│       │           listen("agent:done")            │
│       ▼           listen("agent:error")           │
│  agentStore.messages[]  ←  streaming updates     │
│                                                  │
│  ApprovalModal ──> agentStore.respondApproval()   │
│                      invoke("respond_approval")   │
└──────────────────┬───────────────────────────────┘
                   │  Tauri IPC (invoke / event)
┌──────────────────▼───────────────────────────────┐
│  Backend (Rust / Tauri)                          │
│                                                  │
│  commands::chat::send_message()                  │
│    ├─ Spawns event forwarder (tokio::spawn)      │
│    ├─ Runs AgentLoop::run(content, tx)           │
│    └─ Returns token usage on completion          │
│                                                  │
│  StreamEvent ──> Tauri events emitted to frontend │
│                                                  │
│  commands::chat::stop_generation()                │
│    └─ Sets AtomicBool cancelled flag              │
│                                                  │
│  commands::chat::respond_approval()               │
│    └─ Sends user decision via oneshot channel     │
└──────────────────────────────────────────────────┘
```

---

## 二、Rust 侧实现

### 2.1 `AgentSession` (chat.rs:66-96)

```rust
pub struct AgentSession {
    pub agent: Mutex<AgentLoop>,
    pub cancelled: AtomicBool,
    pub pending_approval_tx: Mutex<Option<oneshot::Sender<bool>>>,
}
```

- `agent`: 包装 xuflow-core 的 AgentLoop，通过 `Mutex` 保证单线程访问
- `cancelled`: 用 `AtomicBool` 替代 `Mutex<bool>`，避免 Clone 问题，允许在 event forwarder 中读取
- `pending_approval_tx`: oneshot 通道的发送端，前端审批结果通过此通道返回

初始化时注册 6 个工具：ReadFile, WriteFile, ListDir, Grep, Bash, WebFetch

### 2.2 `TauriApprovalHandler` (chat.rs:20-47)

实现 xuflow-core 的 `ApprovalHandler` trait：

1. 创建 `oneshot::channel`
2. 将 sender 存入 `pending_tx`
3. 通过 `app_handle.emit("agent:approval-required", ...)` 通知前端
4. 等待前端调用 `respond_approval` 命令回传结果
5. 超时 180 秒自动拒绝

### 2.3 Tauri Commands

| Command | 功能 |
|---------|------|
| `send_message` | 接收用户消息，启动 AgentLoop，通过 mpsc channel 转发 StreamEvent 到前端 |
| `stop_generation` | 设置 cancelled 标志，event forwarder 检测后停止发送事件 |
| `respond_approval` | 接收前端的审批结果（批准/拒绝），通过 oneshot channel 传递给 AgentLoop |

### 2.4 事件流转发 (chat.rs:114-151)

```rust
tokio::spawn(async move {
    while let Some(event) = rx.recv().await {
        if session.cancelled.load(Ordering::SeqCst) { break; }
        match &event {
            StreamEvent::TextDelta { delta } => emit("agent:text-delta", delta),
            StreamEvent::ToolCall { id, name, arguments } => emit("agent:tool-call", json),
            StreamEvent::ToolResult { id, content } => emit("agent:tool-result", json),
            StreamEvent::ApprovalRequired { .. } => emit("agent:approval-required", json),
            StreamEvent::Done { .. } => emit("agent:done", ()),
            StreamEvent::Error { message } => emit("agent:error", message),
        }
    }
});
```

### 2.5 `lib.rs` 更新

```rust
.setup(|app| {
    let session = AgentSession::new(
        String::from(""),          // API key (从配置读取)
        String::from("deepseek-chat"),
        app.handle().clone(),
    );
    app.manage(Arc::new(session));
})
.invoke_handler(tauri::generate_handler![
    commands::chat::send_message,
    commands::chat::stop_generation,
    commands::chat::respond_approval,
])
```

API key 当前为空字符串，后续需从配置文件/环境变量读取。

---

## 三、前端实现

### 3.1 `stores/agent.ts` — Agent Store

**核心方法**：

- `sendMessage(content)`: 添加用户消息到 messages，调用 `invoke("send_message", { content })`，异步等待 Tauri 后端返回
- `stopGeneration()`: 调用 `invoke("stop_generation")` 取消正在运行的 AgentLoop
- `respondApproval(approved)`: 调用 `invoke("respond_approval", { approved })` 回传审批结果

**状态**：
- `messages`: 消息列表（用户 + AI 流式消息）
- `isRunning`: Agent 是否正在运行
- `pendingApproval`: 等待审批的工具信息

### 3.2 `composables/useTauriEvent.ts` — 事件监听

监听 6 种 Tauri 事件并更新 store：

| 事件 | 处理 |
|------|------|
| `agent:text-delta` | 追加增量文本到最后一条 assistant 消息的 content |
| `agent:tool-call` | 在消息中插入工具调用标记 |
| `agent:tool-result` | 在消息中插入可折叠的工具结果 |
| `agent:approval-required` | 设置 store.pendingApproval，触发审批弹窗 |
| `agent:done` | 标记最后一条 assistant 消息为 done，isRunning = false |
| `agent:error` | 在消息末尾追加错误信息，标记 done |

### 3.3 `ChatPanel.vue` — 更新

- `onMounted` 时调用 `setupListeners()` 注册事件监听
- 发送消息走 `store.sendMessage()`（真实 Tauri invoke）
- 输入框在 `store.isRunning` 时禁用
- 运行中显示"停止"按钮，调用 `store.stopGeneration()`

### 3.4 `ApprovalModal.vue` — 更新

- `approve()`: 调用 `store.respondApproval(true)` 回传审批
- `reject()`: 调用 `store.respondApproval(false)` 回传拒绝

### 3.5 `HomeView.vue` — 更新

- 引入 `ApprovalModal` 组件

---

## 四、数据流时序

```
用户输入 "帮我重构这段代码"
  │
  ├─> ChatPanel.sendMessage()
  ├─> agentStore.sendMessage("帮我重构这段代码")
  │     ├─ messages.push({ role: "user", ... })
  │     ├─ messages.push({ role: "assistant", content: "", done: false })
  │     └─ invoke("send_message", { content: "帮我重构这段代码" })
  │
  │  ┌──── Tauri Backend ────┐
  │  │ AgentLoop.run()       │
  │  │   │                   │
  │  │   ├─ TextDelta("好的")──> emit("agent:text-delta", "好的")
  │  │   ├─ TextDelta("，我来")──> emit("agent:text-delta", "，我来")
  │  │   ├─ ToolCall("read_file", ...) ──> emit("agent:tool-call", ...)
  │  │   ├─ ToolResult(...) ──> emit("agent:tool-result", ...)
  │  │   ├─ ApprovalRequired("bash", ...)
  │  │   │     ├─ emit("agent:approval-required", ...)
  │  │   │     └─ [等待 oneshot channel]
  │  │   │          └─ 用户点击批准 ──> invoke("respond_approval", true)
  │  │   ├─ TextDelta("重构完成")──> emit("agent:text-delta", ...)
  │  │   └─ Done ──> emit("agent:done", ())
  │  └────────────────────────┘
  │
  ├─ useTauriEvent 接收所有事件，更新 messages[]
  ├─ MessageItem 实时渲染流式文本
  └─ agent:done ──> isRunning = false
```

---

## 五、待完成

- [ ] API Key 从配置文件/环境变量读取（当前为硬编码空字符串）
- [ ] 模型选择器与后端联动（前端切换模型 → 重建 AgentSession）
- [ ] 消息持久化到 SQLite（利用已有的 SessionStore）
- [ ] 侧边栏项目/会话管理（详见 `docs/UI_LAYOUT_DESIGN.md`）
- [ ] Markdown 渲染增强（markdown-it + highlight.js，StreamText.vue 已预留接口）
- [ ] 导入项目功能实现（Tauri dialog API）
