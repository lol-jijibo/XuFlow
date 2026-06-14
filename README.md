# Xuflow

<p align="center">
  <img src="output/xuflow-logo-blue-bg-clean.png" alt="Xuflow Logo" width="200" />
</p>

<p align="center">
  <strong>CLI + Desktop 双端 AI 编程助手</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.85+-orange?logo=rust" alt="Rust" />
  <img src="https://img.shields.io/badge/Tauri-2.0-blue?logo=tauri" alt="Tauri" />
  <img src="https://img.shields.io/badge/Vue-3.5-green?logo=vue.js" alt="Vue" />
  <img src="https://img.shields.io/badge/Node-20+-339933?logo=node.js" alt="Node" />
  <img src="https://img.shields.io/badge/license-MIT-blue" alt="License" />
</p>

---

## 项目简介

Xuflow 是一个 **CLI + Desktop 双端 AI Agent 编程助手**，支持在终端和桌面 GUI 中使用 AI 进行编程任务。项目采用 **Rust Monorepo** 架构，核心引擎（Agent 循环、LLM 后端、工具系统、安全策略）全部用 Rust 编写，CLI 通过 napi-rs 桥接，桌面端通过 Tauri 直接调用，实现 **"一处编写，双端生效"**。

> 新增任何功能（Provider、工具、安全策略）只需在 Rust Core 中写一次，CLI 和 Desktop 自动同步生效。

---

## 架构概览

```
┌─────────────────────────────────────────────────────────┐
│           Rust 核心库 (xuflow-core) — ★ 唯一真相源 ★     │
│                                                         │
│   Agent循环 / LLM Backend / 工具系统 / 记忆 / 安全策略    │
├──────────────────────────┬──────────────────────────────┤
│     napi-rs 绑定          │       Tauri 直接调用          │
│     (Node.js addon)       │       (Rust crate)           │
├──────────────────────────┼──────────────────────────────┤
│     CLI 入口              │       Desktop 入口            │
│  ┌─────────────────┐     │  ┌────────────────────────┐  │
│  │  Ink TUI (React)  │     │  │  Vue3 + Naive UI       │  │
│  │  终端交互界面      │     │  │  桌面 GUI + 托盘       │  │
│  └─────────────────┘     │  └────────────────────────┘  │
└──────────────────────────┴──────────────────────────────┘
```

---

## 项目结构

```
Xuflow/
├── cli/                    # CLI 入口（Ink TUI + napi-rs 绑定）
│   ├── bin/xuflow.js       # CLI 启动脚本
│   ├── napi/               # napi-rs Rust → Node.js 原生插件
│   └── src/                # Ink React TUI 界面
├── desktop/                # 桌面端入口（Tauri v2 + Vue3）
│   ├── src/                # Vue3 前端（Naive UI + Pinia + Vue Router）
│   ├── src-tauri/          # Tauri Rust 后端（命令、系统托盘）
│   └── index.html
├── packages/
│   └── core/               # Rust 共享核心库
│       └── src/
│           ├── agent/      # Agent 循环、类型定义、系统提示词
│           ├── backends/   # LLM 后端（DeepSeek、火山方舟、OpenAI）
│           ├── tools/      # 工具系统（文件、搜索、Shell、网页）
│           ├── memory/     # 会话存储（SQLite）、向量索引
│           └── config.rs   # Provider/模型配置
├── Cargo.toml              # Rust workspace
├── pnpm-workspace.yaml     # pnpm workspace
└── docs/                   # 设计与实现文档
```

---

## 核心功能

### Agent 循环

基于事件驱动的 Agent 循环，支持流式文本输出、工具调用与审批流程：

```
用户消息 → 构建 messages[] → backend.chat(stream) →
  ├─ text_delta → 推送到前端流式显示
  ├─ tool_use   → 检查危险工具 → 发送审批事件
  ├─ tool_result → 执行结果追加到 messages → 继续循环
  └─ done       → 结束本轮
```

### 多 Provider LLM 后端

| Provider | 说明 |
|----------|------|
| **DeepSeek** | OpenAI 兼容 API |
| **火山方舟** | 字节跳动 VolcEngine |
| **OpenAI 兼容** | 任意 OpenAI 兼容接口，方便接入自定义模型 |

通过 `LlmBackend` trait 可轻松扩展新的 Provider。

### 工具系统

| 工具 | 危险级别 | 说明 |
|------|---------|------|
| `read_file` | 安全 | 读取文件，返回带行号的文本 |
| `write_file` | **危险** | 写入/覆盖文件，需用户审批 |
| `list_dir` | 安全 | 列出目录内容 |
| `grep` | 安全 | 基于 ripgrep 的代码搜索 |
| `bash` | **危险** | Shell 命令执行，含危险命令黑名单拦截 |
| `web_fetch` | 安全 | HTTP 请求 + HTML → Markdown 转换 |

### 安全策略

- 危险工具（`write_file`、`bash`）执行前弹出审批确认
- 命令黑名单：拦截 `rm -rf`、`format` 等危险命令
- 可配置工作目录白名单
- `ApprovalHandler` trait 允许各端注入自己的审批交互

### 会话持久化

基于 SQLite 的会话存储，单文件零配置，支持多会话管理。

---

## 桌面端特性

- **双栏布局**：左侧 260px 侧边栏（项目/会话管理）+ 右侧主内容区
- **项目-会话两级管理**：支持创建/导入项目，每个项目下管理多个会话
- **气泡式聊天界面**：用户消息右侧蓝色气泡，AI 消息左侧灰色卡片气泡
- **流式 Markdown 渲染**：markdown-it + highlight.js，代码块语法高亮 + 一键复制
- **审批弹窗**：危险工具调用时弹出模态确认框
- **模型选择器**：实时切换 LLM Provider 和模型
- **系统托盘**：关闭窗口最小化到托盘，右键菜单（显示/隐藏/新建会话/退出）
- **深色主题**：基于 Naive UI 的完整暗色主题支持

---

## CLI 特性

- **Ink React TUI**：终端内的 React 渲染界面
- **实时流式输出**：AI 回复逐字显示
- **工具调用可视化**：工具名称、参数、结果在终端内展示
- **审批交互**：危险工具通过键盘确认（y/n）
- **多 Provider 支持**：`--provider` 参数切换后端
- **轻量零依赖**：只需 Node.js 即可运行

---

## 快速开始

### 环境要求

- **Rust** 1.85+
- **Node.js** 20+
- **pnpm** 9+
- **Windows**: Microsoft Visual Studio C++ Build Tools（勾选 "Desktop development with C++"）

### 安装依赖

```bash
# 克隆仓库
git clone git@github.com:lol-jijibo/XuFlow.git
cd XuFlow

# 安装 Node 依赖
pnpm install
```

### 运行 CLI

```bash
# 使用 DeepSeek
pnpm dev:cli -- --provider deepseek --api-key <your-key>

# 使用火山方舟
pnpm dev:cli -- --provider volcengine --api-key <your-key>

# 使用 OpenAI 兼容接口
pnpm dev:cli -- --provider openai --api-key <your-key> --base-url https://api.openai.com/v1
```

### 运行桌面端

```bash
pnpm dev:desktop
```

### 构建

```bash
# 构建 CLI
pnpm build:cli

# 构建桌面端安装包
pnpm build:desktop
```

---

## 技术栈

| 层 | 技术 | 说明 |
|----|------|------|
| 共享核心 | Rust | Agent 引擎、LLM 后端、工具系统 |
| CLI 桥接 | napi-rs | Rust → Node.js 原生插件 |
| CLI UI | Ink + React | 终端 TUI 界面 |
| 桌面框架 | Tauri v2 | 跨平台桌面应用框架 |
| 桌面前端 | Vue 3 + Vite | 组合式 API，响应式 UI |
| UI 组件库 | Naive UI | Vue3 原生组件库 |
| 状态管理 | Pinia | Vue 官方状态管理 |
| Markdown | markdown-it + highlight.js | 流式渲染 + 语法高亮 |
| 数据库 | SQLite (rusqlite) | 会话持久化 |
| 包管理 | pnpm workspace | Monorepo 管理 |

---

## 设计原则

1. **Core 零 UI 依赖**：`xuflow-core` 是纯 Rust 库，不依赖任何 UI 框架
2. **一处编写，双端生效**：新增 Provider、工具、安全策略只需在 core 中实现
3. **审批回调注入**：`ApprovalHandler` trait 让各端注入自己的审批 UX（CLI 用 stdin，Desktop 用模态弹窗）
4. **流式事件驱动**：通过 channel 推送事件，前端用 `requestAnimationFrame` 批量更新 DOM
5. **安全第一**：危险命令拦截在 Core 层统一处理，所有端共享同一套安全策略

---

## 截图

<!-- TODO: 添加桌面端和 CLI 的实际运行截图 -->

### 桌面端

> 运行 `pnpm dev:desktop` 后截图，展示聊天界面、侧边栏、审批弹窗等。

### CLI

> 运行 `pnpm dev:cli` 后截图，展示终端内的 TUI 界面、流式输出、工具调用等。

---

## 文档

- [设计方案](DESIGN.md) — 完整的技术架构与设计决策
- [UI 布局设计](docs/UI_LAYOUT_DESIGN.md) — 桌面端 UI 布局方案
- [UI 实现文档](docs/UI_IMPLEMENTATION_2026-06-14.md) — 桌面端 UI 实现细节
- [Tauri 桥接文档](docs/TAURI_BRIDGE_2026-06-14.md) — Rust ↔ 前端通信方案

---

## License

MIT
