# Token 维度的上下文容量管理 — 实现计划 v2

## Context

当前 `AgentLoop.messages` 无限制增长，无上下文窗口感知、无截断、无 token 估算。前端上下文圆环是硬编码占位符，`agent:done` 携带空 payload。目标：端到端 token 感知管理。

## 设计决策（已按优化建议修订）

### 1. Token 估算 — 按模型可配置系数

不使用 tiktoken-rs。字符启发式 + 模型级系数可配置：

```rust
pub struct TokenEstimateConfig {
    /// CJK 字符系数 (默认 1.3)
    pub cjk_coeff: f64,
    /// 非 CJK 字符系数 (默认 0.25)
    pub non_cjk_coeff: f64,
    /// 结构化 JSON / 工具返回内容系数 (默认 0.5，因为符号密集)
    pub structured_coeff: f64,
}
```

- 默认系数适用通用场景；DeepSeek/豆包/GLM 使用相同默认值
- 向 `ModelOption` (config.ts) 添加可选的 `tokenEstimateConfig` 覆盖项，高级用户可调优
- 对工具返回消息（`role: "tool"`）且内容为 JSON 时，自动使用 `structured_coeff`
- `TokenUsage` 事件同时携带 `estimated` 和 `actual`（当 API 返回后），便于前端对比调优

### 2. 截断策略 — 基于 token 量的动态保留

旧方案（固定 3 轮）改为：

- **触发阈值**：用量 ≥ `context_window * 80%`
- **释放目标**：从最早消息逐条丢弃，直到用量降至 `context_window * 60%` 以下
- **保护规则**：无论如何丢弃，保证保留最后 `N` 条 user 消息及其完整关联的 assistant/tool 响应
  - `N` 可配置，默认 3（对应 `min_user_turns: u32`）
- **系统消息保护**：始终保留第一条 system 角色消息（system prompt）
- **原子性**：一轮工具调用（user → assistant(tool_calls) → tool results）不会在中途被腰斩 —— 整轮要么全保留，要么全丢弃
- **保留方向**：从后往前计数 —— 从最新消息回溯，标记第 N 个 user 消息为 "保留起点"，只有起点之前的消息才能被丢弃

### 3. 截断通知 — 静默 + 非侵入式 UI

- **不插入**可见的 system 消息到对话中
- 通过 `StreamEvent::ContextTrimmed` 携带元数据发送给前端
- 前端在页脚上下文圆环旁显示一条短暂的、温和的提示：
  - 圆环旁小标签 "对话已自动整理"（3 秒渐隐）
  - 悬停圆环时显示详情："释放了 X 轮对话，约 Y tokens"
- `ContextTrimmed` 不变为可见消息，完全由前端 UI 层处理

### 4. StreamEvent 变体 — 携带完整元数据

```rust
TokenUsage {
    phase: String,           // "before" | "after"
    estimated: u32,          // 启发式估算值（始终存在）
    actual: Option<u32>,     // API 返回的实际值（仅 phase="after"）
    context_window: u32,     // 当前模型上下文窗口
    context_remaining: u32,  // context_window - max(estimated, actual)
},

ContextTrimmed {
    rounds_removed: u32,     // 丢弃了多少轮对话
    tokens_freed: u32,       // 释放了多少 token
    current_usage_percent: u32,  // 截断后的容量占比 (0-100)
    context_window: u32,
},
```

### 5. agent:done 负载 — 版本化

```json
{
  "v": 1,
  "usage": {
    "prompt_tokens": 12345,
    "completion_tokens": 678,
    "total_tokens": 13023
  }
}
```

- 前端优先检查 `v` 字段：有则解析 usage，无则回退到旧行为（忽略 payload）
- 未来扩展只需递增版本号，前端根据版本选择解析路径

---

## 实现步骤

### 步骤 1：Rust — Token 估算模块 + StreamEvent 扩展

**文件**：[backends/mod.rs](packages/core/src/backends/mod.rs)

在文件末尾添加新模块 `pub mod token_counter`：

```rust
pub struct TokenEstimateConfig {
    pub cjk_coeff: f64,          // 默认 1.3
    pub non_cjk_coeff: f64,      // 默认 0.25
    pub structured_coeff: f64,   // 默认 0.5 — 用于 JSON/工具返回
}

impl Default for TokenEstimateConfig { ... }

pub fn estimate_tokens(text: &str, config: &TokenEstimateConfig) -> u32 { ... }
pub fn is_cjk_char(c: char) -> bool { ... }
pub fn is_structured_content(content: &serde_json::Value) -> bool { ... }
pub fn estimate_message_tokens(msg: &ChatMessage, config: &TokenEstimateConfig) -> u32 { ... }
pub fn default_context_window(model_id: &str) -> u32 { ... }
```

更新 `StreamEvent` 枚举 —— 替换旧的简单变体为：

```rust
TokenUsage {
    phase: String,           // "before" | "after"
    estimated: u32,
    actual: Option<u32>,
    context_window: u32,
    context_remaining: u32,
},
ContextTrimmed {
    rounds_removed: u32,
    tokens_freed: u32,
    current_usage_percent: u32,
    context_window: u32,
},
```

### 步骤 2：Rust — AgentLoop 动态截断

**文件**：[loop_.rs](packages/core/src/agent/loop_.rs)

`AgentLoop` 新增字段：
```rust
context_window: u32,           // 当前上下文窗口大小
min_user_turns: u32,           // 保留的最小 user 轮数（默认 3）
token_config: TokenEstimateConfig,  // 估算系数
```

新增方法：
- `set_context_window(w: u32)` / `context_window() -> u32`
- `set_min_user_turns(n: u32)`
- `estimate_total_tokens(&self) -> u32`
- `trim_context(&mut self, tx: &mpsc::Sender<StreamEvent>)` — 动态截断逻辑

`trim_context` 算法：
1. 计算 `estimated = estimate_total_tokens()`
2. 若 `estimated < context_window * 80%`，直接返回
3. 从第一条系统消息之后开始，将消息组织为**不可分割的轮次**（turn = user 消息 + 后续所有 assistant/tool 消息直到下一条 user 为止）
4. 从后往前数，标记最后 `min_user_turns` 个轮次为**保护轮次**
5. 从最旧的未保护轮次开始逐个丢弃，每丢弃一轮重新估算，直到 `estimated < context_window * 60%` 或只剩保护轮次
6. 发送 `StreamEvent::ContextTrimmed { rounds_removed, tokens_freed, current_usage_percent, context_window }`
7. 不插入系统消息到 `self.messages`

修改 `run()` 方法：
- 在推入用户消息后，发送 `TokenUsage { phase: "before", ... }`
- 调用 `trim_context()`
- 在每轮 API 返回累积 usage 后，发送 `TokenUsage { phase: "after", actual: Some(total), ... }`
- 在下一轮循环开始前（若还有工具调用），再次发送 `TokenUsage { phase: "before", ... }`

### 步骤 3：Tauri — 事件转发 + 新命令

**文件**：[chat.rs](desktop/src-tauri/src/commands/chat.rs)

**事件转发器更新**（替换第 212-213 行的 `StreamEvent::Done` 分支，新增 token 分支）：

```rust
StreamEvent::TokenUsage { phase, estimated, actual, context_window, context_remaining } => {
    let payload = serde_json::json!({
        "phase": phase,
        "estimated": estimated,
        "actual": actual,
        "context_window": context_window,
        "context_remaining": context_remaining,
    });
    let _ = app_clone.emit("agent:token-usage", payload.to_string());
}
StreamEvent::ContextTrimmed { rounds_removed, tokens_freed, current_usage_percent, context_window } => {
    let payload = serde_json::json!({
        "rounds_removed": rounds_removed,
        "tokens_freed": tokens_freed,
        "current_usage_percent": current_usage_percent,
        "context_window": context_window,
    });
    let _ = app_clone.emit("agent:context-trimmed", payload.to_string());
}
StreamEvent::Done { usage } => {
    let payload = serde_json::json!({
        "v": 1,
        "usage": {
            "prompt_tokens": usage.prompt_tokens,
            "completion_tokens": usage.completion_tokens,
            "total_tokens": usage.total_tokens,
        }
    });
    let _ = app_clone.emit("agent:done", payload.to_string());
}
```

新增 Tauri 命令：
- `set_context_window(context_window: u32)` — 设置上下文窗口
- `set_min_user_turns(min_turns: u32)` — 设置最小保留轮数

**文件**：[lib.rs](desktop/src-tauri/src/lib.rs) — 注册 `set_context_window`、`set_min_user_turns`

### 步骤 4：前端 — Config Store 扩展

**文件**：[config.ts](desktop/src/stores/config.ts)

```typescript
export interface TokenEstimateConfig {
  cjkCoeff: number;        // 默认 1.3
  nonCjkCoeff: number;     // 默认 0.25
  structuredCoeff: number; // 默认 0.5
}

export interface ModelOption {
  // ... existing fields ...
  contextWindow?: number;             // 模型上下文窗口，默认 128000
  tokenEstimateConfig?: TokenEstimateConfig;  // 可选覆盖
}
```

持久化新增字段：
```typescript
contextWindows: Record<string, number>;         // 用户自定义上下文窗口
minUserTurns: number;                            // 最小保留轮数，默认 3
tokenEstimateConfigs: Record<string, TokenEstimateConfig>;  // 用户自定义系数
```

新增 computed：`activeMinUserTurns`、`activeTokenEstimateConfig`
新增方法：`setContextWindow()`、`setMinUserTurns()`、`setTokenEstimateConfig()`

### 步骤 5：前端 — Agent Store Token 状态

**文件**：[agent.ts](desktop/src/stores/agent.ts)

```typescript
// Token 追踪
const tokenUsage = ref<number>(0);          // 当前估算用量
const tokenActual = ref<number | null>(null); // API 返回的实际用量
const contextWindow = ref<number>(128000);
const contextRemaining = ref<number>(128000);
const tokenUsagePercent = computed(() => ...);
const tokenWarningLevel = computed(() => ...); // green/yellow/orange/red

// 截断通知
const contextTrimmed = ref<boolean>(false);
const trimMeta = ref<{ roundsRemoved: number; tokensFreed: number } | null>(null);
```

### 步骤 6：前端 — TauriEvent 处理

**文件**：[useTauriEvent.ts](desktop/src/composables/useTauriEvent.ts)

新增 `agent:token-usage` 监听器：
```typescript
listen<string>("agent:token-usage", (event) => {
  const d = JSON.parse(event.payload);
  if (d.phase === "before") {
    agentStore.tokenUsage = d.estimated;
  } else if (d.phase === "after" && d.actual != null) {
    agentStore.tokenActual = d.actual;
    // 使用实际值校准估算
    agentStore.tokenUsage = Math.max(d.estimated, d.actual);
  }
  agentStore.contextWindow = d.context_window;
  agentStore.contextRemaining = d.context_remaining;
});
```

新增 `agent:context-trimmed` 监听器：
```typescript
listen<string>("agent:context-trimmed", (event) => {
  const d = JSON.parse(event.payload);
  agentStore.contextTrimmed = true;
  agentStore.trimMeta = { roundsRemoved: d.rounds_removed, tokensFreed: d.tokens_freed };
  agentStore.tokenUsagePercent = d.current_usage_percent;
  // 3 秒后自动隐藏
  setTimeout(() => { agentStore.contextTrimmed = false; }, 3000);
});
```

修改 `agent:done` 监听器（版本感知）：
```typescript
listen<string>("agent:done", (event) => {
  try {
    const p = JSON.parse(event.payload);
    if (p.v === 1 && p.usage) {
      agentStore.tokenUsage = p.usage.total_tokens;
      agentStore.tokenActual = p.usage.total_tokens;
    }
    // v 未知或缺失 → 优雅忽略
  } catch { /* 旧版空 payload，安全跳过 */ }
  // ... 现有 done 逻辑不变 ...
});
```

### 步骤 7：前端 — ChatPanel 上下文圆环 + 截断提示

**文件**：[ChatPanel.vue](desktop/src/components/chat/ChatPanel.vue)

移除硬编码占位符（第 51-56 行），替换为：
```typescript
const tokenPercent = computed(() => agentStore.tokenUsagePercent);
const tokenWarningLevel = computed(() => agentStore.tokenWarningLevel);
const tokenRemaining = computed(() => agentStore.contextRemaining);
```

更新 SVG 上下文圆环：前景弧线 `stroke-dasharray` 基于 `tokenPercent`，`stroke` 基于 `tokenWarningLevel` 着色（周长 39.27）。

上下文圆环旁新增**截断提示标签**：
```html
<span v-if="agentStore.contextTrimmed" class="trim-badge">
  对话已自动整理
  <span v-if="agentStore.trimMeta" class="trim-detail">
    (释放 {{ agentStore.trimMeta.roundsRemoved }} 轮)
  </span>
</span>
```
CSS：`opacity: 1 → 0`，3 秒 `transition`，`pointer-events: none`。

工具提示更新：
```html
已用 {{ tokenPercent }}% · 剩余 {{ formatTokens(contextRemaining) }}
<!-- 悬停详情：估算值 vs 实际值对比 -->
```

### 步骤 8：前端 — StatusBar + 设置 UI

**文件**：[StatusBar.vue](desktop/src/components/layout/StatusBar.vue)
- 右侧添加 Token 标签：`{{ formatK(tokenUsage) }} / {{ formatK(contextWindow) }}`
- 颜色编码遵循 `tokenWarningLevel`

**文件**：[SettingsView.vue](desktop/src/views/SettingsView.vue)
- menuOptions 中 "接入点管理" 之前新增 `{ label: "上下文管理", key: "context" }`

**文件**：[SettingsPanel.vue](desktop/src/components/config/SettingsPanel.vue)
- 新增 `v-if="activeSection === 'context'"` 区块：
  - **上下文窗口**：每个模型一个 `NInputNumber`，允许覆盖（1K–1M，步长 1000）
  - **最小保留轮数**：`NSlider`（1–10，默认 3）
  - **高级：估算系数**：可折叠区域，展示 `tokenEstimateConfig` 每个模型的 CJK/非 CJK/结构化系数微调输入
  - 所有变更自动持久化到 localStorage，并 push 到 Rust 后端

---

## 验证

1. **估算准确性**：发送中英混合 + 代码块消息。对比控制台中 `TokenUsage.before.estimated` 与 `TokenUsage.after.actual`，偏差应 <30%
2. **上下文圆环**：多条消息后圆环从绿色(12点)顺时针填充 → 黄色 → 橙色 → 红色。悬停查看详细 tooltip
3. **动态截断**：设置 contextWindow=2000 并发送长消息。验证 `agent:context-trimmed` 触发，UI 短暂显示"对话已自动整理"，圆环百分比回落至 60% 以下
4. **轮次保护**：min_user_turns=2，发送 5 条 user 消息。验证截断后保留了最近 2 个完整轮次
5. **版本化 agent:done**：旧版前端（空 payload）不崩溃；新版正确解析 `{v:1, usage:{...}}`
6. **设置持久化**：修改上下文窗口和系数 → 重启 → 验证值保留
7. **构建**：`cargo build` + 前端 typecheck
