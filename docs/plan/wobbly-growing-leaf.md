# 右侧代码审查侧边栏 - 实现方案

## Context

Xuflow 目前已具备左侧项目/会话侧边栏 + 中间聊天面板的布局，但缺少代码审查能力。用户希望参照 OpenAI Codex 桌面端的右侧审查面板，在 Xuflow 中实现类似的代码审查侧边栏功能。

参考 Codex 的审查面板核心能力：文件变更列表、内联 diff 查看、行级注释、Git 暂存/回退、AI 审查触发、以及与 Agent 对话的反馈闭环。

## 目标

在 HomeView 右侧新增一个可切换的审查侧边栏，包含：
1. **Diff 查看器** - 变更文件列表、展开式内联 diff、语法高亮
2. **Git 暂存管理** - 按文件/按块暂存、取消暂存、回退、提交
3. **AI 审查** - 触发 Agent 分析 diff，返回结构化审查结果
4. **行级评论** - 在 diff 行上添加评论，发送回 Agent 进行修复

## 架构总览

```
HomeView.vue (三栏布局)
├── Sidebar (左, 260px) - 已有
├── .home-main (中, flex:1) - 已有
│   ├── ChatPanel
│   └── StatusBar
├── ReviewPanel (右, ~320px, 可切换) - 新增 ⭐
│   ├── ReviewToolbar (范围选择 + 操作)
│   ├── ReviewFileList (文件树)
│   │   └── ReviewDiffItem (单文件 diff, 可展开)
│   └── ReviewActions (暂存/回退/提交)
└── ApprovalModal - 已有
```

## 实现阶段

### 阶段 1：右侧侧边栏外壳 + 切换机制

**文件修改：**
- `desktop/src/views/HomeView.vue` - 添加 ReviewPanel 组件, 三栏布局
- `desktop/src/components/review/ReviewPanel.vue` - 新建，侧边栏容器
- `desktop/src/stores/review.ts` - 新建，审查状态管理 store
- `desktop/src/components/layout/StatusBar.vue` - 添加审查面板切换按钮

**关键设计：**
- 使用 Pinia store (`useReviewStore`) 管理面板可见性
- 侧边栏宽度 320px，从右侧滑入动画
- 复用项目现有的 dark mode 模式（`.dark` class + CSS 变量）
- 切换按钮放在 StatusBar 右侧，显示变更文件数量徽章

### 阶段 2：Diff 查看器核心

**文件修改/新建：**
- `desktop/src/components/review/ReviewFileList.vue` - 文件变更列表
- `desktop/src/components/review/ReviewDiffItem.vue` - 单文件 diff 展开
- `desktop/src/components/review/ReviewDiffHunk.vue` - diff 块渲染
- `desktop/src/utils/diffParser.ts` - git diff 输出解析器
- `desktop/src/composables/useGitReview.ts` - git 操作 composable

**关键技术：**
- 通过 Tauri invoke 调用已有的 `git_diff` + `git_status` 命令获取原始数据
- 前端解析 unified diff 格式为结构化数据：
  ```ts
  interface DiffFile {
    path: string; status: 'added' | 'modified' | 'deleted';
    additions: number; deletions: number;
    hunks: DiffHunk[];
  }
  interface DiffHunk {
    header: string;
    lines: DiffLine[];
  }
  interface DiffLine {
    type: 'add' | 'remove' | 'context';
    oldLineNo?: number; newLineNo?: number;
    content: string; comments: Comment[];
  }
  ```
- **语法高亮**：复用已有的 `highlight.js`（与 StreamText.vue 一致），支持 190+ 语言
- **主流开发语言全覆盖：**

  | 类别 | 语言 |
  | ---- | ---- |
  | 前端 | TypeScript, JavaScript, Vue, React JSX/TSX, HTML, CSS/SCSS/Less |
  | 后端 | Rust, Go, Python, Java, Kotlin, C#, PHP, Ruby |
  | 系统 | C, C++, Zig, Assembly (x86/ARM) |
  | 数据 | JSON, YAML, TOML, XML, SQL, GraphQL, Protobuf |
  | 脚本 | Bash/Shell, PowerShell, Lua, Perl |
  | 配置 | Dockerfile, Makefile, CMake, Nginx, HCL (Terraform) |
  | 文档 | Markdown, LaTeX, reStructuredText |

- **语言检测方式：**
  - 文件扩展名映射：`.vue` → `xml`（Vue SFC）, `.rs` → `rust`, `.tsx` → `typescript` 等
  - `diffParser.ts` 内置 `EXTENSION_LANG_MAP` 映射表（覆盖 80+ 扩展名）
  - fallback 策略：未识别扩展名 → 自动检测代码特征 → 纯文本
- **Diff 行高亮处理：**
  - 先按文件语言对 diff hunk 中的代码行做语法标记（tokenize）
  - 剥离 `+`/`-`/` ` 前缀后进行高亮，再将前缀重新拼接
  - 新增行（`+`）和删除行（`-`）的背景色独立于语法颜色，两者叠加不冲突
  - 上下文行（` `）正常高亮，无背景色
- 展开/折叠动画参考 PlanApprovalCard 的 frosted glass 样式

### 阶段 3：Git 暂存与回退操作

**文件修改/新建：**
- `desktop/src/components/review/ReviewActions.vue` - 批量操作工具栏
- `desktop/src/composables/useGitReview.ts` - 扩展 git 操作

**操作能力：**
| 层级 | 操作 | Tauri invoke |
|------|------|-------------|
| 全部 | 暂存所有 / 回退所有 | `git_add` / `git_checkout` (新增) |
| 文件 | 暂存文件 / 取消暂存 / 回退文件 | `git_add` / `git_reset` (新增) |
| 块级 | 暂存块 / 回退块 | `git_apply` (新增) |

**需要在 Rust 后端新增的命令：**
- `git_unstage(path)` - 取消暂存单个文件
- `git_revert_file(path)` - 回退文件到 HEAD
- `git_stage_hunk(path, hunk_index)` - 暂存单个 diff 块
- `git_revert_hunk(path, hunk_index)` - 回退单个 diff 块
- `git_commit(message)` - 提交（已有工具，需注册为命令）

### 阶段 4：AI 审查集成

**文件修改/新建：**
- `packages/core/src/tools/review.rs` - 新的 `code_review` 工具
- `packages/core/src/backends/mod.rs` - 新增 `ReviewFindings` StreamEvent 变体
- `desktop/src/composables/useTauriEvent.ts` - 监听 `agent:review-findings` 事件
- `desktop/src/stores/review.ts` - 存储审查结果
- `desktop/src/components/review/ReviewComment.vue` - 行级评论组件

**工作流：**
1. 用户在审查面板点击 "AI 审查" 按钮
2. 前端将当前 diff 内容作为消息发送给 Agent（或通过快捷操作注入）
3. Agent 调用 `code_review` 工具（或直接分析 diff）
4. 审查结果通过 `agent:review-findings` 事件流式返回
5. 前端在 diff 行上渲染评论标记

**`code_review` 工具定义：**
```rust
// 参数
{
  "diff_content": "string",     // git diff 输出
  "focus": ["bugs", "security", "perf", "style"],  // 审查维度
  "file_paths": ["string"]      // 聚焦的文件
}
// 返回
{
  "findings": [
    {
      "file": "string",
      "line_start": number,
      "line_end": number,
      "severity": "error" | "warning" | "info",
      "category": "bug" | "security" | "perf" | "style",
      "title": "string",
      "description": "string",
      "suggestion": "string"  // 可选，修复建议
    }
  ],
  "summary": "string"
}
```

### 阶段 5：行级评论与反馈闭环

**文件修改/新建：**
- `desktop/src/components/review/ReviewCommentInput.vue` - 行内评论输入框
- `desktop/src/stores/review.ts` - 评论状态管理

**交互流：**
1. 鼠标悬停在 diff 行上 → 显示 "+" 按钮
2. 点击 "+" → 弹出评论输入框
3. 输入评论 → 提交 → 存储到该行的 `comments` 数组
4. 用户点击 "发送评论给 Agent" → 将评论注入到当前聊天会话
5. Agent 读取评论，针对性修复代码
6. 修复后 diff 自动刷新

### 阶段 6：范围选择器

**功能：**
- **未提交变更**（默认）：`git diff` + `git diff --cached`
- **分支变更**：`git diff origin/main...HEAD`
- **最近一轮变更**：Agent 最后修改的文件列表

**实现：**
- `ReviewToolbar` 中的下拉选择器
- 切换范围时重新获取 diff
- "最近一轮" 通过追踪 Agent 工具调用中的文件写入来实现

## 文件清单

### 新建文件（11 个）
```
desktop/src/
├── components/review/
│   ├── ReviewPanel.vue          # 侧边栏主容器
│   ├── ReviewToolbar.vue        # 范围选择 + 批量操作
│   ├── ReviewFileList.vue       # 文件列表
│   ├── ReviewDiffItem.vue       # 单文件 diff 展开
│   ├── ReviewDiffHunk.vue       # 单个 diff 块
│   ├── ReviewActions.vue        # 底部操作栏
│   ├── ReviewComment.vue        # 行级评论显示
│   └── ReviewCommentInput.vue   # 行级评论输入
├── stores/
│   └── review.ts                # 审查状态 Pinia store
├── composables/
│   └── useGitReview.ts          # Git 操作 composable
└── utils/
    └── diffParser.ts            # Unified diff 解析器
```

### 修改文件（6 个）
```
desktop/src/
├── views/HomeView.vue           # 添加 ReviewPanel 到三栏布局
├── components/layout/StatusBar.vue  # 添加审查面板切换按钮
├── composables/useTauriEvent.ts     # 监听 review-findings 事件
└── stores/agent.ts              # 暴露 diff 上下文给 agent
packages/core/src/
├── backends/mod.rs              # 添加 ReviewFindings StreamEvent
└── tools/mod.rs                 # 注册 code_review 工具
```

### Rust 后端新增（可选，阶段 3 需要）
```
packages/core/src/tools/review.rs       # code_review 工具
desktop/src-tauri/src/commands/git.rs   # git_unstage, git_revert_file, git_stage_hunk, git_revert_hunk
```

## 与现有系统集成

- **主题**：复用 `useThemeStore().isDark`，所有组件使用 `.dark` class 模式
- **持久化**：审查评论复用 `SessionStore` SQLite，新增 `review_comments` 表
- **事件系统**：新增 `agent:review-findings` 事件，遵循 `useTauriEvent.ts` 模式
- **Agent 工具**：`code_review` 工具遵循现有 `Tool` trait 模式
- **UI 样式**：复用 Sidebar 的边框/阴影/过渡模式，PlanApprovalCard 的 frosted glass 效果，ToolCallCard 的展开/折叠模式

## 验证方式

1. **侧边栏切换**：点击 StatusBar 按钮 → 右侧面板滑入/滑出
2. **Diff 查看 + 语法高亮**：修改文件后 → 审查面板显示变更，文件列表正确，行级加减高亮；`.rs`/`.vue`/`.ts`/`.py`/`.go` 等主流文件类型均正确着色，暗色/亮色模式下高亮颜色协调
3. **Git 暂存**：暂存单个文件 → `git status` 确认已暂存
4. **AI 审查**：点击审查 → Agent 分析 diff → 结构化结果以行级评论展示
5. **评论反馈**：添加评论 → 发送给 Agent → Agent 针对性修复
6. **端到端**：修改代码 → 审查面板查看 diff → AI 审查发现问题 → 手动/自动修复 → 暂存 → 提交
