# Agent 循环功能改进计划

## Context

Xuflow 当前的 Agent 循环（`src/ui/useAgent.ts` 中的 `sendMessage`）存在多个关键缺陷：无上下文窗口管理导致长对话超出模型限制、无重试逻辑导致网络波动直接失败、达到 MAX_LOOPS 后静默失败无用户反馈、token 统计已计算但从未显示、工具结果更新直接修改 state 违反 React 不可变性、两个后端 90% 代码重复、无法取消进行中的操作、独立工具调用串行执行浪费性能、无循环检测可能导致 LLM 无限重复调用同一工具。

本次改进旨在系统性地修复这些问题，使 Agent 循环更加健壮、高效和用户友好。

## 改动文件清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/retry.ts` | **新建** | 指数退避重试包装器 |
| `src/tokenUtils.ts` | **新建** | Token 估算 + 上下文窗口截断 |
| `src/loopDetector.ts` | **新建** | 工具循环调用检测 |
| `src/backends/shared.ts` | **新建** | 提取 DeepSeek/VolcEngine 共享流式逻辑 |
| `src/ui/useAgent.ts` | **修改** | 核心循环重构（集成上述模块） |
| `src/ui/app.tsx` | **修改** | 恢复 Header、接入 totalTokens、Ctrl+C 取消 |
| `src/backends/deepseek.ts` | **修改** | 简化为薄配置包装 |
| `src/backends/volcengine.ts` | **修改** | 简化为薄配置包装 |
| `src/types.ts` | **修改** | ChatParams 可选扩展 temperature/topP |

---

## Phase 1: 新建工具模块

### Step 1 — `src/retry.ts`：指数退避重试

- 导出 `withRetry<T>(fn, options?): AsyncGenerator<T>`
- 重试条件：网络错误（ECONNREFUSED, ECONNRESET, ETIMEDOUT, ENOTFOUND）、HTTP 429/5xx
- 不退重：HTTP 400/401/403/404/422
- 退避策略：`baseDelayMs * 2^attempt` + 0-300ms 随机抖动
- 默认：maxRetries=3, baseDelayMs=1000
- 支持 AbortSignal，中断后立即停止重试
- 错误检测通过解析 OpenAI SDK 抛出的 error.message 中的 HTTP 状态码

### Step 2 — `src/tokenUtils.ts`：上下文窗口管理

- `estimateTokens(text)`：英文 `ceil(len/4)`，CJK 占比 >30% 时 `ceil(len/2.5)`
- `estimateMessageTokens(msg)`：含角色标记固定开销（4 tokens）
- `truncateMessages(messages, systemPrompt, maxTokens, reserveTokens)`：
  - systemPrompt 始终保留
  - 从最新到最旧按"轮次"（user → assistant → tool results）累积
  - 整轮保留或整轮丢弃，防止孤立的 tool_calls
  - 如果最新一轮就超出预算，保留 systemPrompt + 该轮（工具结果截断到 2000 字符）
- `getModelContextLimit(model)`：返回模型上下文窗口大小，默认 128K
- 在每轮 while 循环调用 `backend.chat()` 之前截断，不修改 `llmMessagesRef.current`

### Step 3 — `src/loopDetector.ts`：循环调用检测

- `createLoopDetector(maxRepeat=3)` 返回 `{ record, reset }`
- 维护滑动窗口，每项为 `{name, argsFingerprint}`
- `argsFingerprint` = `JSON.stringify(args, Object.keys(args).sort())`
- 连续相同 (name, fingerprint) 达到 maxRepeat 时 `record()` 返回 true
- `reset()` 在每次 `sendMessage()` 开始时调用

---

## Phase 2: 核心循环重构 (`src/ui/useAgent.ts`)

### Step 4 — AbortController 取消支持

- 新增 `abortRef = useRef<AbortController | null>(null)`
- `sendMessage()` 开始时创建新 AbortController
- 传递给 `withRetry()` 和长时间工具执行
- 新增 `cancel()` 函数暴露给 UI 层
- `finally` 块中清空 `abortRef.current`

### Step 5 — 集成重试逻辑

将 `backend.chat({...})` 包装为：
```ts
withRetry(() => backend.chat({...}), { maxRetries: 3, signal: abortRef.current?.signal })
```

### Step 6 — 集成上下文截断

在 `backend.chat()` 调用前：
```ts
const truncatedMessages = truncateMessages(
  llmMessagesRef.current, systemPrompt,
  getModelContextLimit(backend.model), 4096
);
```
将 `truncatedMessages` 传给 API，`llmMessagesRef.current` 继续完整增长。

### Step 7 — MAX_LOOPS 用户反馈

while 循环结束后，若 `loop >= MAX_LOOPS`，追加系统消息告知用户并建议缩小任务范围或输入"继续"。

### Step 8 — 修复 State 不可变性

两处工具结果更新从 `find` + 直接属性赋值改为 `map` + 展开运算符创建新对象。

### Step 9 — 并行工具执行

- 安全工具（read_file, list_dir, grep）用 `Promise.all` 并行执行
- 危险工具（bash, write_file）保持串行（审批流程依赖）
- 状态更新使用函数式 `setMessages` 避免竞态

### Step 10 — 集成循环检测

在工具执行前调用 `loopDetector.record()`，检测到循环后标记工具为 error 状态、追加系统消息、退出循环。

---

## Phase 3: UI + 后端变更

### Step 11 — `src/ui/app.tsx`：恢复 Header + 取消支持

- 从 `useAgent()` 解构 `totalTokens` 和 `cancel`
- 在主视图顶部渲染 `<Header>`（组件已存在，只是从未被渲染）
- 添加 Ctrl+C 处理器：当 status !== "idle" 时调用 `cancel()`

### Step 12 — `src/backends/shared.ts`：提取共享逻辑

提取三个函数：
- `createOpenAIStreamingGenerator(client, params)`：核心流式生成逻辑
- `toOpenAI(msg)`：消息格式转换
- `toOpenAITool(tool)`：工具定义转换

### Step 13 — 简化后端文件

`deepseek.ts` 和 `volcengine.ts` 各缩减为 ~25 行的薄配置包装，仅保留构造函数（baseURL、默认模型）和一行 `yield* createOpenAIStreamingGenerator(...)`。

### Step 14 — `src/types.ts`：可选扩展

`ChatParams` 新增可选字段 `temperature?: number` 和 `topP?: number`，向后兼容。

---

## 验证

1. **构建**：`npm run build` 通过
2. **现有测试**：`node --import tsx test/inputCursorLayout.test.ts` 和 `test/renderAlternateScreen.test.ts` 通过
3. **重试逻辑**：模拟网络错误，验证 3 次重试 + 指数退避
4. **上下文截断**：构造 200+ 条消息，验证 API 收到截断后的消息列表
5. **MAX_LOOPS**：临时设 MAX_LOOPS=2，验证系统消息出现
6. **Token 显示**：Header 渲染正确，每次 LLM 响应后更新
7. **并行执行**：请求同时读取多个文件，验证并发执行
8. **循环检测**：构造重复工具调用场景，验证 3 次后中断
9. **取消**：发起长请求后 Ctrl+C，验证状态回到 idle 并显示取消消息
10. **后端一致性**：两个后端 smoke test 验证行为一致
