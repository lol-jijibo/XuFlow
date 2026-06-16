# 桌面端 UI 配色优化方案

## Context

当前 Xuflow 桌面端以 **indigo（靛蓝）色系**为主色调（`#6366f1`、`#4f46e5`、`#818cf8`），用户感知为紫色。需求是将整体配色改为**偏灰色系**，同时让左侧项目列表中的**每个项目名以白亮色显示**。

## 涉及文件

核心修改集中在以下文件：

1. **[theme.ts](packages/desktop/src/stores/theme.ts)** — Naive UI 主题色配置
2. **[Sidebar.vue](packages/desktop/src/components/layout/Sidebar.vue)** — 侧边栏（项目列表）
3. **[MessageItem.vue](packages/desktop/src/components/chat/MessageItem.vue)** — 消息气泡渐变
4. **[ChatPanel.vue](packages/desktop/src/components/chat/ChatPanel.vue)** — 聊天面板交互色
5. **[StreamText.vue](packages/desktop/src/components/chat/StreamText.vue)** — Markdown 渲染中的链接/代码色
6. **[SettingsPanel.vue](packages/desktop/src/components/config/SettingsPanel.vue)** — 设置页图标/链接色
7. **[ApprovalModal.vue](packages/desktop/src/components/approval/ApprovalModal.vue)** — 审批弹窗工具名色

## 配色方案

### 新主色调：灰色系 (Gray/Slate)

| 用途 | 旧值 (Indigo) | 新值 (Gray) |
|------|-------------|-----------|
| Primary | `#6366f1` | `#6b7280` |
| Primary Hover | `#818cf8` | `#9ca3af` |
| Primary Pressed | `#4f46e5` / `#4338ca` | `#4b5563` |
| Primary Suppl | `rgba(99,102,241,0.15)` | `rgba(107,114,128,0.15)` |

### 侧边栏项目名：白亮色

- 亮色模式：`#1e293b`（深灰，保持可读性，因为背景是 `#fafafa`）
- 暗色模式：`#f1f5f9`（亮白灰，在 `#101014` 背景上突出显示）

### 渐变替换

- AI 头像渐变：`linear-gradient(135deg, #6b7280, #9ca3af)` 替代 `linear-gradient(135deg, #6366f1, #8b5cf6)`
- 用户消息气泡：`linear-gradient(135deg, #4b5563, #6b7280)` 替代 `linear-gradient(135deg, #6366f1, #7c3aed)`
- 阴影色同步替换为灰色系

## 具体修改

### 1. theme.ts — Naive UI 主题覆盖

- Light: `primaryColor: "#6b7280"`, `primaryColorHover: "#9ca3af"`, `primaryColorPressed: "#4b5563"`, `primaryColorSuppl: "rgba(107,114,128,0.08)"`
- Dark: `primaryColor: "#9ca3af"`, `primaryColorHover: "#b0b7c3"`, `primaryColorPressed: "#6b7280"`, `primaryColorSuppl: "rgba(156,163,175,0.15)"`

### 2. Sidebar.vue — 项目名白亮色 + 交互色

- `.project-name` 暗色模式：`color: #f1f5f9`（白亮）
- `.project-name` 亮色模式：`color: #1e293b`（深灰黑，保持可读）
- `.project-item.active > .project-row` 背景改为灰色系：`rgba(107, 114, 128, 0.08)` / dark `rgba(156, 163, 175, 0.15)`
- `.project-icon` 颜色改为灰色：`#6b7280` / dark `#9ca3af`
- `.conversation-item.active` 同上灰色背景
- 其他 indigo 引用全部替换为 gray 对应值

### 3. MessageItem.vue — 头像/气泡渐变

- `.avatar-ai` background: `linear-gradient(135deg, #6b7280, #9ca3af)`, box-shadow 改为灰色
- `.bubble-user` background: `linear-gradient(135deg, #4b5563, #6b7280)`, box-shadow 改为灰色
- `.dot` 打字指示器颜色: `#6b7280` / dark `#9ca3af`

### 4. ChatPanel.vue — 交互色

- `.prompt-card:hover` border-color: `#9ca3af`, box-shadow 改为灰色
- `.prompt-card svg` color: `#6b7280`
- `.input-wrapper:focus-within` border-color: `#9ca3af`, box-shadow 改为灰色
- dark 模式对应值同步

### 5. StreamText.vue — Markdown 元素色

- `code:not(.hljs)` background: `rgba(107, 114, 128, 0.1)`, color: `#6b7280`
- dark: `rgba(156, 163, 175, 0.2)`, color: `#9ca3af`
- `blockquote` border-left: `#6b7280`, background: `rgba(107, 114, 128, 0.04)`
- `a` color: `#6b7280`

### 6. SettingsPanel.vue

- `.card-header svg` color: `#6b7280`
- `.endpoint-hint a` color: `#6b7280`
- 反馈文字 `#6366f1` → `#6b7280`

### 7. ApprovalModal.vue

- `.tool-name` color: `#6b7280`

## 验证方式

1. `cd D:\Projects-star\Xuflow-desktop && pnpm run dev`（或对应启动命令）启动桌面端
2. 检查亮色/暗色两种模式下的视觉效果：
   - 侧边栏项目名在暗色模式下应为白亮色
   - 所有原本紫色/靛蓝的地方应变为灰色系
   - 消息气泡、AI 头像渐变、输入框聚焦边框等
3. 切换亮/暗主题，确认过渡自然
