# 修复"停止生成"按钮无响应的问题

## Context

用户点击"停止生成"按钮时，UI 没有任何反应——生成仍在继续，`isRunning` 一直为 `true`，按钮虽然可见但点击无效。这是因为停止机制只在 Tauri 事件转发层设置了 `AtomicBool` 标记，而核心的 `AgentLoop` 完全不感知这个标记，继续运行直到自然结束。

## 根因分析

完整的停止链条有三个环节，目前仅第一个环节生效：

1. **Frontend → Tauri 命令** ✅: `stopGeneration()` → `invoke("stop_generation")` → 设置 `cancelled = true`，正常工作
2. **事件转发器 (Forwarder)** ⚡: 转发器检查 `cancelled` 后 break，**但 break 前不发射 `agent:done` 事件**
3. **AgentLoop 核心** ❌: `loop_.rs` 的 `run()` 方法完全没有检查取消信号。它继续：
   - 接收后端事件并通过 `tx.send(event).await.ok()` 发送（`.ok()` 静默吞掉因通道关闭产生的错误）
   - 执行工具调用（bash/file 等，可能耗时很长）
   - 最多循环 10 轮（`MAX_TOOL_ROUNDS`）
   - **`send_message` Tauri 命令不会返回，直到 agent loop 完成**

结果：转发器停止发事件 → 前端文字停止更新 → 但 `isRunning` 一直为 true，`send_message` 不返回 → 用户点击停止按钮无反馈。

## 修改方案

### 文件 1: `packages/core/src/agent/loop_.rs`

**改动**: 给 `run()` 方法增加 `cancelled: &AtomicBool` 参数，在关键检查点检测取消。

具体检查点：
1. 每一轮 tool round 开始时（`for _round` 循环顶部）
2. 每次从 `backend_rx.recv()` 收到事件后
3. 每执行一个工具前

当检测到取消时：
- 调用 `chat_handle.abort()` 立即中止后端 HTTP 请求
- 发送 `StreamEvent::Done` 并携带当前 usage
- 提前 return

```rust
use std::sync::atomic::{AtomicBool, Ordering};

pub async fn run(&mut self, user_message: String, tx: mpsc::Sender<StreamEvent>, cancelled: &AtomicBool) -> Result<Usage, anyhow::Error> {
    // ...
    for _round in 0..MAX_TOOL_ROUNDS {
        // [检查点] 每轮开始前检查取消
        if cancelled.load(Ordering::SeqCst) {
            tx.send(StreamEvent::Done { usage: total_usage.clone() }).await.ok();
            return Ok(total_usage);
        }
        
        // ... 创建 backend_tx 通道，发起 chat 任务 ...
        
        while let Some(event) = backend_rx.recv().await {
            // [检查点] 每次收到事件后检查取消
            if cancelled.load(Ordering::SeqCst) {
                chat_handle.abort();
                tx.send(StreamEvent::Done { usage: total_usage.clone() }).await.ok();
                return Ok(total_usage);
            }
            // ... 处理事件 ...
        }
        
        // ... 收集 tool calls ...
        
        // [检查点] 执行每个工具前检查
        for (tool_id, tool_name, tool_args) in &tool_calls {
            if cancelled.load(Ordering::SeqCst) {
                // 跳过剩余工具，发送 Done 返回
                tx.send(StreamEvent::Done { usage: total_usage.clone() }).await.ok();
                return Ok(total_usage);
            }
            // ... 执行工具 ...
        }
    }
}
```

`abort()` 会向 tokio 任务发送取消信号。`reqwest` 的 HTTP 流在下一个 `.await` 点会收到此信号并抛出 `JoinError`，从而关闭 HTTP 连接。不需要额外添加依赖——`AtomicBool` 是 `std` 类型。

### 文件 2: `desktop/src-tauri/src/commands/chat.rs`

**改动 A**: 在 `send_message` 中把 `state.cancelled` 传给 `agent_guard.run()`

```rust
// 原代码
agent_guard.run(content, tx).await

// 新代码
agent_guard.run(content, tx, &state.cancelled).await
```

**改动 B**: 转发器在因取消而 break 时，先发送 `agent:done` 事件，让前端知道生成已停止

```rust
while let Some(event) = rx.recv().await {
    if session.cancelled.load(Ordering::SeqCst) {
        let _ = app_clone.emit("agent:done", ());  // 新增：通知前端生成已停止
        break;
    }
    // ... 转发事件 ...
}
```

### 文件 3: `desktop/src/stores/agent.ts`

**改动**: 在 `sendMessage` 的 `finally` 块中，确保最后一个 assistant 消息标记为 `done`，这样无论 AgentLoop 如何结束（取消、错误、正常完成），UI 都能正确显示完成状态。

```typescript
finally {
  // 确保最后一个 assistant 消息被标记为完成（特别是被取消时 event:done 可能丢失）
  const msgs = conv.messages;
  const lastMsg = msgs[msgs.length - 1];
  if (lastMsg && lastMsg.role === "assistant" && !lastMsg.done) {
    lastMsg.done = true;
  }
  projectStore.persistMessages();
  isRunning.value = false;
}
```

## 不修改的文件

- **`packages/core/Cargo.toml`**: `AtomicBool` 来自 `std`，无需新增依赖
- **`desktop/src-tauri/Cargo.toml`**: 同上
- **所有前端组件**: 不需要改动 UI 组件，只改 store 逻辑

## 验证方法

1. **编译检查**: `cd packages/core && cargo build` 确保 Rust 编译通过
2. **Tauri 编译**: `cargo build` (在 desktop 目录或根目录) 确保桌面端编译通过
3. **功能验证**: 启动桌面应用，发送一条消息，在 AI 回复过程中点击停止按钮——预期行为：
   - 按钮点击后，文字立即停止更新
   - 几秒内 `isRunning` 变为 false
   - 输入框重新可用
   - 停止的消息显示完成状态（没有闪烁的输入指示器）
4. **边界情况**: 连续快速点击停止按钮（幂等性）；在工具执行期间停止；在审批弹窗期间停止

## 改动文件清单

| 文件 | 改动类型 |
|---|---|
| `packages/core/src/agent/loop_.rs` | 核心逻辑——添加取消检测点 |
| `desktop/src-tauri/src/commands/chat.rs` | 传递取消标记 + 转发器发送 `agent:done` |
| `desktop/src/stores/agent.ts` | finally 块添加完成标记 |
