# 桌面端 UI 布局 & 消息气泡样式 设计方案

## Context

当前桌面端布局是简单的 TitleBar + ChatPanel + StatusBar 三段式，缺少侧边栏/项目/会话管理。需要在保留现有假数据流的前提下，重构整体布局，引入 **项目-会话** 两级结构。

## 涉及文件

| 文件 | 操作 |
|---|---|
| `packages/desktop/src/components/layout/Sidebar.vue` | **新建** — 侧边栏（Logo + 项目列表 + 设置入口） |
| `packages/desktop/src/stores/project.ts` | **新建** — 项目 & 会话两级状态管理 |
| `packages/desktop/src/views/HomeView.vue` | **重写** — sidebar + 主内容区双栏布局 |
| `packages/desktop/src/components/chat/ChatPanel.vue` | **微调** — 适配新主内容区 |
| `packages/desktop/src/components/layout/TitleBar.vue` | **微调** — 移到主内容区顶部 |
| `packages/desktop/src/components/chat/MessageItem.vue` | **重写** — 气泡样式 |
| `packages/desktop/src/components/chat/StreamText.vue` | **增强** — 接入 markdown-it + highlight.js |
| `packages/desktop/src/App.vue` | **微调** — 全局样式 |
| `packages/desktop/src/views/SettingsView.vue` | **微调** — 适配新布局 |

---

## 一、整体布局

```
┌────────────┬─────────────────────────────────────────┐
│ 🔷 Xuflow  │  TitleBar                                │
│            │  当前项目名 | 模型选择器                   │
│ ────────── ├─────────────────────────────────────────┤
│ 项目 ≣ + 导入│                                         │
│            │          ChatPanel                      │
│ 📁 项目A   │                                         │
│    💬 会话1 │   ┌───────────────────────┐             │
│    💬 会话2 │   │  你: 帮我重构这段代码    │ (右侧蓝泡)  │
│ 📁 项目B   │   └───────────────────────┘             │
│    💬 会话1 │   ┌────────────────────────────┐        │
│            │   │ X: 好的，我来重构...       │ (左灰泡) │
│            │   │ ```rust\nfn main()...```  │          │
│            │   └────────────────────────────┘        │
│            │                                         │
│            ├─────────────────────────────────────────┤
│ ⚙️ 设置     │  StatusBar  ● 就绪 | tokens: 0        │
└────────────┴─────────────────────────────────────────┘
```

---

## 二、侧边栏 (`Sidebar.vue`)

### 布局（从上到下）

1. **Logo 区域**（左上角，固定）：小号图标(24x24) + "Xuflow" 文字(13px)，可点击返回首页，整体紧凑

2. **项目区 header**（固定）：
   - 左侧：标题 "项目"
   - 右侧：三个操作按钮 — "全部收起"、"＋ 新建"、"导入"
   - "全部收起" 和 "+ 新建"、"导入" **默认隐藏，鼠标悬停该行时才显示**
   - "全部收起" 悬停时显示 tooltip "全部收起"（Naive UI NButton + NTooltip）
   - 点击"全部收起" → 折叠所有已展开的项目，只显示项目名

3. **项目列表**（中间，可滚动）：
   - 每个项目项：折叠/展开箭头 + 项目名称
   - 展开时显示该项目下的会话列表（缩进）
   - 当前活跃会话高亮
   - 悬停会话项显示删除按钮
   - 当前项目高亮

4. **底部**（固定）：
   - 设置按钮（⚙️图标 + "设置"，跳转到 /settings）

### 交互

- 点击项目名 → 展开/折叠会话列表，同时切换活跃项目
- 点击会话名 → 切换活跃会话，切换到该会话的消息
- 鼠标悬停 "项目" 行 → 显示 "全部收起"、"+ 新建" 和 "导入" 按钮
- 点击 "+ 新建" → 弹出小输入框创建新项目
- 点击 "导入" → 打开系统文件夹选择对话框（通过 Tauri dialog API，暂时用 prompt 模拟）
- 项目展开后，项目名旁显示 + 按钮用于在该项目下新建会话

### 样式

- 宽度 260px，暗色背景 `var(--n-color-embedded)`
- Logo 区域高度约 36px，padding 紧凑
- 项目 header 行高度 32px，按钮用 `NButton size="tiny" text`
- 活跃项背景色：`var(--n-primary-color-suppl)`
- 分隔线用 `var(--n-border-color)`

---

## 三、项目 & 会话 Store (`project.ts`)

```ts
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
  path?: string;        // 本地项目路径（导入时设置）
  source: 'local' | 'imported';
  conversations: Conversation[];
  createdAt: number;
  updatedAt: number;
}
```

提供：
- `projects: Ref<Project[]>`
- `activeProjectId: Ref<string | null>`
- `activeConversationId: Ref<string | null>`
- `activeProject` (computed)
- `activeConversation` (computed)
- `activeMessages` (computed) — 当前活跃会话的消息列表
- `createProject(name)` — 新建项目
- `importProject(name, path)` — 导入本地项目
- `deleteProject(id)` — 删除项目
- `createConversation(projectId)` — 在某项目下新建会话
- `deleteConversation(projectId, convId)` — 删除会话
- `switchTo(projectId, convId)` — 切换活跃项目+会话
- 初始化时自动创建一个默认项目和默认会话

---

## 四、消息气泡样式

### 用户消息（右侧）
- 右对齐 flex-end，蓝色主题气泡
- `background: var(--n-primary-color)`，白色文字
- border-radius: 12px 12px 4px 12px
- 最大宽度 70%
- Avatar（小圆圈，首字母）在右侧

### AI 消息（左侧）
- 左对齐 flex-start，浅灰卡片气泡
- `background: var(--n-card-color)`
- border: 1px solid var(--n-border-color)
- border-radius: 12px 12px 12px 4px
- 最大宽度 85%
- Avatar（小圆圈，X）在左侧
- 内容支持 Markdown 渲染（markdown-it + highlight.js）

### 代码块
- 深色背景 `#1e1e1e`（独立于主题）
- 顶部 header 栏：语言名称（左） + 复制按钮（右）
- highlight.js 语法高亮，github-dark 主题
- border-radius: 8px，margin: 8px 0

### 工具调用卡片
- 内嵌在 AI 消息中
- 虚线边框，浅色背景
- 显示：图标 + 工具名 + 参数摘要
- 可折叠查看结果

---

## 五、实现步骤

1. **新建 `stores/project.ts`** — 项目+会话两级状态管理
2. **新建 `Sidebar.vue`** — 完整侧边栏组件（Logo + 项目列表 + 会话列表 + 底部操作 + 设置入口）
3. **重写 `HomeView.vue`** — 改为 sidebar + 主内容区（TitleBar + ChatPanel + StatusBar）
4. **调整 `TitleBar.vue`** — 移到主内容区，显示当前项目名，保留模型选择器
5. **重写 `MessageItem.vue`** — 完整气泡样式
6. **增强 `StreamText.vue`** — markdown-it + highlight.js 真实 Markdown 渲染
7. **更新 `agent.ts`** — messages 改为从 project store 的 activeMessages 读取
8. **微调 `App.vue` / `SettingsView.vue`** — 适配新布局

---

## 六、验证

```bash
cd packages/desktop
pnpm dev          # 检查 UI 布局
pnpm tauri dev    # 完整桌面端
```

检查点：
- 侧边栏：项目列表展开/折叠、新建/删除项目、会话切换
- Logo 点击返回首页
- 设置按钮跳转设置页
- 消息气泡方向正确（用户右/AI 左）
- Markdown 代码块高亮正常
- 导入项目弹窗交互
