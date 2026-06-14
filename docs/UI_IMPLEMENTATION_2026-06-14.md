# 桌面端 UI 布局 & 消息气泡样式 实现文档

**日期**: 2026-06-14
**涉及模块**: `packages/desktop/src/`

---

## 一、整体布局

```
┌────────────┬─────────────────────────────────────────┐
│ 🔷 Xuflow  │  TitleBar                                │
│            │  默认项目 / 默认会话      [模型选择器]     │
│ ────────── ├─────────────────────────────────────────┤
│ 项目 ≣+导入│                                         │
│            │          ChatPanel                      │
│ 📁 默认项目 │                                         │
│   💬 默认会话│  ┌───────────────────────┐             │
│            │  │  你: 帮我重构这段代码    │ (右侧蓝泡)  │
│            │  └───────────────────────┘             │
│            │  ┌────────────────────────────┐        │
│            │  │ X: 好的，我来重构...       │ (左灰泡) │
│            │  │ ```rust\nfn main()...```  │          │
│            │  └────────────────────────────┘        │
│            │                                         │
│            ├─────────────────────────────────────────┤
│ ⚙️ 设置     │  StatusBar  ● 就绪 | 默认项目 / 默认会话 │
└────────────┴─────────────────────────────────────────┘
```

双栏布局：左侧 260px 侧边栏 + 右侧弹性主内容区。

---

## 二、新建/修改文件清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `stores/project.ts` | **新建** | 项目 & 会话两级状态管理 |
| `components/layout/Sidebar.vue` | **新建** | 侧边栏（Logo + 项目列表 + 设置入口） |
| `views/HomeView.vue` | **重写** | sidebar + 主内容区双栏布局 |
| `views/SettingsView.vue` | **重写** | 同样引入 sidebar 双栏布局 |
| `components/layout/TitleBar.vue` | **重写** | 显示当前项目名/会话名，保留模型选择器 |
| `components/layout/StatusBar.vue` | **微调** | 显示当前项目/会话路径 |
| `components/chat/MessageItem.vue` | **重写** | 气泡样式（用户右蓝泡，AI 左灰泡） |
| `components/chat/StreamText.vue` | **重写** | markdown-it + highlight.js 渲染 |
| `components/chat/ChatPanel.vue` | **微调** | messages 改为 computed，适配新 store |
| `stores/agent.ts` | **重写** | messages 委托给 project store 的 activeConversation |
| `composables/useTauriEvent.ts` | **微调** | 事件写入 project store 的 activeConversation |
| `App.vue` | **微调** | 全局样式增加 background/color |

---

## 三、项目 & 会话 Store (`stores/project.ts`)

### 数据结构

```ts
interface ChatMessage {
  role: "user" | "assistant" | "system";
  content: string;
  done: boolean;
}

interface Conversation {
  id: string;
  title: string;
  messages: ChatMessage[];
  createdAt: number;
  updatedAt: number;
}

interface Project {
  id: string;
  name: string;
  path?: string;           // 本地项目路径（导入时设置）
  source: "local" | "imported";
  conversations: Conversation[];
  createdAt: number;
  updatedAt: number;
}
```

### 提供的状态和方法

**状态**:
- `projects: Ref<Project[]>` — 所有项目
- `activeProjectId / activeConversationId` — 当前活跃 ID
- `activeProject` (computed) — 当前活跃项目对象
- `activeConversation` (computed) — 当前活跃会话对象
- `activeMessages` (computed) — 当前活跃会话的消息列表

**方法**:
- `createProject(name)` / `importProject(name, path)` / `deleteProject(id)`
- `createConversation(projectId, title?)` / `deleteConversation(projectId, convId)`
- `switchTo(projectId, convId?)` — 切换活跃项目+会话

**初始化**: 自动创建"默认项目" + "默认会话"

---

## 四、侧边栏 (`components/layout/Sidebar.vue`)

### 布局（从上到下）

1. **Logo 区域** — SVG 图标(24x24) + "Xuflow" 文字(13px)，点击返回首页
2. **分隔线**
3. **项目区 header** — 左侧"项目"标题，右侧三个按钮（默认隐藏，hover 显示）：
   - "全部收起"（≣）— 带 NTooltip 提示"全部收起"
   - "＋ 新建" — 弹出内联输入框创建项目
   - "导入" — 尝试 Tauri dialog API，失败则 fallback 到 prompt
4. **项目列表**（可滚动）：
   - 项目行：折叠箭头 + 项目名，点击展开/折叠 + 切换活跃项目
   - 展开时：会话列表（缩进），💬 图标 + 会话名
   - 活跃项目/会话高亮（`var(--n-primary-color-suppl)`）
   - 悬停会话项显示删除按钮
   - 项目展开后项目名旁显示 + 按钮用于新建会话
5. **分隔线**
6. **底部** — 设置按钮（⚙️ + "设置"，跳转 /settings）

### 样式
- 宽度 260px，`background: var(--n-color-embedded)`
- 右侧边框 `border-right: 1px solid var(--n-border-color)`

---

## 五、消息气泡样式 (`components/chat/MessageItem.vue`)

### 用户消息（右侧）
- `flex-direction: row-reverse`，右对齐
- 蓝色气泡：`background: var(--n-primary-color)`，白色文字
- `border-radius: 12px 12px 4px 12px`
- 最大宽度 70%
- 圆形 Avatar（U）在右侧

### AI 消息（左侧）
- `flex-direction: row`，左对齐
- 灰色卡片气泡：`background: var(--n-card-color)` + `border: 1px solid var(--n-border-color)`
- `border-radius: 12px 12px 12px 4px`
- 最大宽度 85%
- 圆形 Avatar（X）在左侧
- 内容为空且未完成时显示打字动画（三个跳动圆点）

---

## 六、Markdown 渲染 (`components/chat/StreamText.vue`)

- 使用 **markdown-it** 渲染，配置：`html: false, linkify: true, breaks: true`
- 使用 **highlight.js** 语法高亮，主题 `github-dark.css`（已在 main.ts 引入）

### 代码块
- 深色背景 `#1e1e1e`（独立于主题）
- 顶部 header 栏：语言名称（左） + 复制按钮（右）
- 代码区：`padding: 12px`，`overflow-x: auto`

### 其他元素
- 行内代码：`background: var(--n-color-embedded)`，圆角
- 引用块：左侧 3px 主题色边框
- 表格：完整边框样式
- 链接：主题色

---

## 七、agent store 重构 (`stores/agent.ts`)

`messages` 从独立 `ref` 改为 `computed`，委托给 project store：

```ts
const messages = computed({
  get: () => useProjectStore().activeMessages,
  set: () => { /* no-op */ },
});
```

- `sendMessage()` 直接写入 `projectStore.activeConversation.messages`
- `useTauriEvent` 的事件处理也改为写入 `projectStore.activeConversation.messages`
- ChatPanel 中 `store.messages` 直接使用（computed 自动解包）

---

## 八、其他微调

### TitleBar.vue
- 移除了"会话"/"设置"导航按钮（侧边栏已有）
- 左侧显示 `项目名 / 会话名`
- 保留模型选择器

### StatusBar.vue
- 增加当前项目/会话路径显示

### HomeView.vue
- 从垂直三段式改为水平双栏：`Sidebar` + `主内容区(NLayout)`
- 主内容区包含 TitleBar + ChatPanel + StatusBar

### SettingsView.vue
- 同样引入 Sidebar 双栏布局

### App.vue
- 全局样式增加 `background` 和 `color`

---

## 九、验证

```bash
cd packages/desktop
pnpm dev          # Vite dev server 检查 UI
pnpm tauri dev    # 完整桌面端
```

编译检查：
- TypeScript: `vue-tsc --noEmit` ✓
- Rust: `cargo check -p xuflow-desktop` ✓

---

## 十、数据流

```
Sidebar 点击会话
  → projectStore.switchTo(projectId, convId)
  → activeConversation 更新
  → agentStore.messages (computed) 自动指向新会话的 messages
  → ChatPanel 重新渲染消息列表

用户发送消息
  → ChatPanel.sendMessage()
  → agentStore.sendMessage(text)
  → 写入 activeConversation.messages
  → invoke("send_message") → Rust AgentLoop
  → Tauri events → useTauriEvent
  → 更新 activeConversation.messages (流式追加)
  → MessageItem + StreamText 实时渲染
```
