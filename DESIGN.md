# Xuflow 设计方案（Monorepo：CLI + Desktop 共享核心）

## 一、项目定位

将 Xuflow 打造为 **CLI + Desktop 双端 Agent 工具**，共享同一套 Rust 核心引擎，UI 层各自独立。

- **CLI**：Ink (React TUI)，终端内运行，适合 SSH/轻量场景
- **Desktop**：Tauri v2 + Vue3，桌面 GUI + 系统托盘常驻，适合日常使用

**新增任何功能（Provider、工具、安全策略）只需在 Rust Core 中写一次，双端自动生效。**

---

## 二、技术架构

### 核心思路：Monorepo + Rust 共享核心

**CLI 和 Desktop 共享同一套 Rust Agent 引擎**，只有 UI 层不同。新增功能只需写一次，两个端同时生效。

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
│  │  (UI 层, 轻量)    │     │  │  (UI 层, 完整)         │  │
│  └─────────────────┘     │  └────────────────────────┘  │
├──────────────────────────┴──────────────────────────────┤
│                  共享基础设施                             │
│  系统提示词 / 工具定义 / 模型列表 / 安全策略 — 全部在 core │
└─────────────────────────────────────────────────────────┘
```

### 为什么这样设计？

| 场景 | 分开维护（两套代码） | 共享核心（本方案） |
|---|---|---|
| 新增 LLM Provider | TS 写一遍 + Rust 写一遍 | **Rust 写一遍**，两边自动生效 |
| 新增工具 | 写两遍，行为可能不一致 | **写一遍**，行为完全一致 |
| 修改 Agent 循环 | 改两处，容易遗漏 | **改一处**，两个端同步 |
| 修改安全策略 | 可能 CLI 修了 Desktop 没修 | **改一处** |
| 模型列表更新 | 改两个文件 | **改一处** |

### 关键技术决策

| 层 | 选型 | 理由 |
|---|---|---|
| 共享核心 | Rust crate (xuflow-core) | 性能好，Tauri 可直接依赖，napi-rs 可暴露给 Node.js |
| CLI 桥接 | napi-rs | 将 Rust 核心编译为 Node.js addon，TS 直接 import |
| CLI UI | Ink (React TUI) — 保持不变 | 只改 UI 调用层，UI 组件不动 |
| 桌面框架 | Tauri v2 | 直接依赖 xuflow-core crate，零开销调用 |
| 桌面前端 | Vue3 + Vite + TypeScript | 生态成熟，组合式API灵活 |
| UI 组件库 | Naive UI | Vue3 原生，Agent UI 场景丰富 |
| 状态管理 | Pinia | Vue 官方推荐，类型友好 |
| Markdown 渲染 | markdown-it + highlight.js | 轻量，流式渲染友好 |



---

## 三、项目结构（Monorepo）

```
xuflow/                              # 一个 Git 仓库
├── Cargo.toml                       # Rust workspace
├── package.json                     # pnpm workspace root
├── pnpm-workspace.yaml
│
├── packages/
│   ├── core/                        # ★ Rust 核心库 — 唯一真相源 ★
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs               # 库入口，暴露所有公共 API
│   │       ├── agent/
│   │       │   ├── mod.rs
│   │       │   ├── loop.rs          # Agent 循环（核心）
│   │       │   ├── types.rs         # Agent 类型定义
│   │       │   └── system_prompt.rs # 系统提示词
│   │       ├── backends/
│   │       │   ├── mod.rs           # LlmBackend trait + 工厂
│   │       │   ├── deepseek.rs      # DeepSeek
│   │       │   ├── volcengine.rs    # 火山方舟
│   │       │   └── openai_compat.rs # 通用 OpenAI 兼容后端
│   │       ├── tools/
│   │       │   ├── mod.rs           # Tool trait + 注册表 + 安全策略
│   │       │   ├── file.rs          # read_file / write_file / list_dir
│   │       │   ├── grep.rs          # ripgrep 搜索
│   │       │   ├── bash.rs          # Shell 执行 + 危险命令拦截
│   │       │   └── web.rs           # Web 浏览
│   │       ├── memory/
│   │       │   ├── mod.rs
│   │       │   ├── session.rs       # 会话存储（SQLite）
│   │       │   └── vector.rs        # 向量索引（可选）
│   │       └── config.rs            # 配置模型（Provider/Model/API Key）
│   │
│   ├── cli/                         # CLI 入口（TypeScript）
│   │   ├── package.json
│   │   ├── tsconfig.json
│   │   ├── bin/
│   │   │   └── xuflow.js            # CLI 启动脚本
│   │   ├── src/
│   │   │   ├── loop.ts              # 入口：加载 napi 绑定，启动 Agent
│   │   │   └── ui/                  # Ink TUI 界面（保持不变）
│   │   │       ├── app.tsx
│   │   │       ├── useAgent.ts      # 改为调用 napi 绑定的 Agent
│   │   │       ├── inputCursor.ts
│   │   │       └── workspaceStatus.ts
│   │   └── napi/                    # napi-rs 绑定层
│   │       ├── Cargo.toml           # 依赖 xuflow-core
│   │       ├── src/
│   │       │   └── lib.rs           # #[napi] 导出给 Node.js
│   │       └── index.js             # 生成的 JS 入口
│   │
│   └── desktop/                     # 桌面端入口（Tauri + Vue3）
│       ├── package.json
│       ├── index.html
│       ├── vite.config.ts
│       ├── src/                     # Vue3 前端
│       │   ├── App.vue
│       │   ├── main.ts
│       │   ├── router/
│       │   ├── stores/              # Pinia
│       │   │   ├── agent.ts
│       │   │   ├── config.ts
│       │   │   └── tray.ts
│       │   ├── components/
│       │   │   ├── chat/
│       │   │   │   ├── ChatPanel.vue
│       │   │   │   ├── MessageItem.vue
│       │   │   │   ├── StreamText.vue
│       │   │   │   └── ToolCallCard.vue
│       │   │   ├── approval/
│       │   │   │   └── ApprovalModal.vue
│       │   │   ├── config/
│       │   │   │   ├── ModelSelector.vue
│       │   │   │   └── SettingsPanel.vue
│       │   │   └── layout/
│       │   │       ├── TitleBar.vue
│       │   │       └── StatusBar.vue
│       │   ├── views/
│       │   │   ├── HomeView.vue
│       │   │   └── SettingsView.vue
│       │   └── composables/
│       │       └── useTauriEvent.ts
│       ├── src-tauri/               # Tauri Rust 后端
│       │   ├── Cargo.toml           # 依赖 xuflow-core
│       │   ├── tauri.conf.json
│       │   ├── capabilities/
│       │   │   └── default.json
│       │   ├── icons/
│       │   └── src/
│       │       ├── main.rs
│       │       ├── lib.rs           # Tauri 命令注册
│       │       ├── commands/
│       │       │   ├── mod.rs
│       │       │   ├── chat.rs      # 调用 xuflow-core 的 Agent
│       │       │   ├── config.rs
│       │       │   └── tray.rs
│       │       └── tray.rs          # 系统托盘逻辑
│       └── tsconfig.json
│
└── docs/                            # 文档
    └── DESIGN.md
```

### 依赖关系

```
xuflow-core (Rust)         ← 所有核心逻辑在这里
    ↑               ↑
    │               │
napi 绑定        Tauri 直接依赖
    │               │
    ↓               ↓
CLI (TS+Ink)    Desktop (Tauri+Vue3)
```

- **xuflow-core** 不依赖任何 UI 框架，纯 Rust 库
- **CLI** 通过 napi-rs 将 core 编译为 `.node` addon，TypeScript 直接 `import`
- **Desktop** 的 `src-tauri` 在 `Cargo.toml` 中 `xuflow-core = { path = "../../core" }`，零开销调用
- 两个端的 UI 各自独立，但底层 Agent 行为**完全一致**

---

## 四、核心模块设计

### 4.1 Agent 循环（Rust 实现）

对应 CLI 的 `useAgent.ts` 中的 while 循环：

```
用户消息 → 构建 messages[] → backend.chat(stream) →
  ├─ text_delta → 推送到前端流式显示
  ├─ tool_use   → 检查危险工具 → 发送审批事件到前端 →
  │                执行工具 → 结果追加到 messages → 继续循环
  └─ done        → 结束本轮，显示完成
```

**与前端通信方式**：
- 前端调用 `invoke('send_message', { content })` 启动 Agent
- Agent 通过 **Tauri Event** 实时推送流式事件到前端：
  - `agent:text-delta` — 文本增量
  - `agent:tool-call` — 工具调用请求
  - `agent:tool-result` — 工具执行结果
  - `agent:approval-required` — 需要用户审批
  - `agent:done` — 本轮完成
  - `agent:error` — 错误

### 4.2 工具系统

CLI 已有 5 个工具，桌面端再增加 1 个：

| 工具 | 危险 | 说明 |
|---|---|---|
| `read_file` | 否 | 读取文件，返回带行号的文本 |
| `write_file` | **是** | 写入/覆盖文件，需审批 |
| `list_dir` | 否 | 列出目录 |
| `grep` | 否 | ripgrep 搜索 |
| `bash` | **是** | Shell 执行，有危险命令拦截 |
| `web_fetch` | 否 | **新增**，HTTP 请求 + HTML→Markdown |

### 4.3 LLM Backend

保留 CLI 的 Provider 抽象，用 Rust trait 实现：

```rust
#[async_trait]
pub trait LlmBackend: Send + Sync {
    fn model(&self) -> &str;
    async fn chat(
        &self,
        params: ChatParams,
        tx: Sender<StreamEvent>,
    ) -> Result<Usage>;
}
```

支持的 Provider（与 CLI 一致）：
- DeepSeek（OpenAI 兼容 API）
- VolcEngine 火山方舟（OpenAI 兼容 API）
- 扩展：任何 OpenAI 兼容接口（方便用户接入自定义模型）

### 4.4 记忆/持久化

CLI 用 PostgreSQL + Qdrant。桌面端简化为：

- **会话历史**：**SQLite**（通过 `tauri-plugin-sql`），单文件，零配置
- **向量检索**：可选。初版可做简单的文本搜索，后续接入本地向量库（如 `usearch`）
- **配置**：存储在 Tauri app data 目录 (`$APPDATA/xuflow/`)

### 4.5 系统托盘

```
托盘菜单：
├── 显示/隐藏主窗口
├── 新建会话
├── ──────────
├── 退出
```

- 关闭窗口 → 隐藏到托盘（不退出）
- 托盘图标点击 → 显示/隐藏窗口
- 右键托盘 → 显示菜单

### 4.6 审批机制

对应 CLI 的 `DANGEROUS_TOOLS` 审批：

- 当 Agent 要执行 `write_file` 或 `bash` 时，前端弹出审批对话框
- 显示：工具名、参数、具体命令/内容
- 选项：批准 / 拒绝 / 批准本次会话内所有同类操作
- 超时 3 分钟自动拒绝

---

## 五、分阶段实施计划

### Phase 0：Monorepo 初始化（0.5 周）

1. 创建 Rust workspace (`Cargo.toml`)
2. 创建 pnpm workspace (`pnpm-workspace.yaml`)
3. 创建 `packages/core/` Rust crate 骨架
4. 创建 `packages/cli/` 从现有 Xuflow 项目迁入
5. 创建 `packages/desktop/` Tauri + Vue3 项目骨架
6. 验证 workspace 构建链路通

**产出**：Monorepo 结构就绪，`cargo build` + `pnpm install` 正常

### Phase 1：Rust Core 实现（2 周）

> ⭐ **最关键阶段**：这个做完，CLI 和 Desktop 都有了核心引擎

1. 实现 `LlmBackend` trait + DeepSeek/VolcEngine/OpenAI 兼容 backend
2. 实现工具系统（file/grep/bash/web）+ 安全策略 + 危险命令拦截
3. 实现 Agent 循环（状态机、工具调用、审批回调）
4. 实现会话存储（SQLite）
5. 单元测试覆盖

**产出**：`xuflow-core` crate 可独立运行测试

### Phase 2：CLI 接入 Rust Core（1 周）

1. 用 napi-rs 搭建绑定层 (`packages/cli/napi/`)
2. 将 Core 的 Agent 暴露为 Node.js 模块
3. 改造 CLI 的 `useAgent.ts`，从直接调 OpenAI SDK → 调 napi Agent
4. CLI 的 Ink TUI 保持不变
5. 回归测试，确保 CLI 行为不变

**产出**：CLI 正常运行，底层已切换为 Rust Core

### Phase 3：Desktop GUI（2 周）

1. 配置 Vue3 + Naive UI + Pinia + Vue Router
2. 实现聊天面板 UI（消息列表 + 输入框 + 流式文本）
3. 实现 Markdown 代码块渲染（语法高亮）
4. Tauri Command 层（调用 xuflow-core 的 Agent）
5. Tauri Event 流式推送 → 前端实时渲染
6. 审批弹窗（write_file/bash 确认）
7. 模型选择器 + 设置面板
8. 系统托盘（最小化到托盘、显示/隐藏/退出）

**产出**：桌面端完整可用

### Phase 4：打磨发布（1 周）

1. 错误处理和边界情况
2. 窗口状态记忆（位置、大小）
3. 打包配置（Windows NSIS 安装包）
4. 自动更新（Tauri updater）
5. 图标和品牌
6. CLI + Desktop 双端联调

**产出**：双端都可分发的正式版本

---

## 六、前期准备工作清单

### 环境准备

- [ ] 安装 Rust 工具链：`winget install Rustlang.Rustup` 或 https://rustup.rs
- [ ] 安装 Node.js 20 LTS
- [ ] 安装 Microsoft Visual Studio C++ Build Tools（勾选 "Desktop development with C++"）
- [ ] 验证环境：`rustc --version`、`cargo --version`、`node --version`

### Monorepo 初始化

```bash
# 1. 创建 workspace 根目录
mkdir xuflow && cd xuflow

# 2. Rust workspace
cargo init --lib packages/core
# 编辑根 Cargo.toml 为 workspace

# 3. pnpm workspace
pnpm init
# 创建 pnpm-workspace.yaml:
# packages:
#   - "packages/*"

# 4. 将现有 CLI 项目迁入
cp -r ../Xuflow packages/cli

# 5. 创建 Desktop 项目
cd packages/desktop
npm create tauri-app@latest .
# 交互选择：TypeScript, Vue, pnpm

# 6. 安装前端依赖
pnpm add naive-ui pinia vue-router markdown-it highlight.js @tauri-apps/api @tauri-apps/plugin-sql
pnpm add -D @types/markdown-it

# 7. 验证全链路
cargo build          # Rust 编译
pnpm install         # 前端依赖
pnpm --filter desktop tauri dev  # 启动桌面端
```

### 根 Cargo.toml (workspace)

```toml
[workspace]
resolver = "2"
members = [
    "packages/core",
    "packages/cli/napi",
    "packages/desktop/src-tauri",
]
```

### 根 pnpm-workspace.yaml

```yaml
packages:
  - "packages/*"
```

### 从 CLI 项目可复用的资产

| 资产 | 复用方式 |
|---|---|
| 系统提示词 | 迁入 `packages/core/src/agent/system_prompt.rs`，CLI 和 Desktop 都从这里读 |
| 工具定义（name/description/parameters） | 翻译为 Rust struct，在 core 中定义 |
| 危险命令黑名单 | 迁入 `packages/core/src/tools/bash.rs` |
| Backend 流式解析逻辑 | 参考 TS 代码，用 reqwest + serde 重写 |
| 模型列表（modelConfig.ts） | 迁入 `packages/core/src/config.rs`，CLI napi 导出给 TS，Desktop 直接读 |
| Ink TUI 界面 | 保留在 `packages/cli/src/ui/`，只改 useAgent.ts 的调用层 |

---

## 七、架构注意事项

1. **Core 不依赖任何 UI**：`xuflow-core` 是纯 Rust 库，不依赖 Tauri、napi-rs 或任何 UI 框架。它只暴露 Rust trait 和 struct。

2. **Agent 状态机**：Agent 循环是异步任务。Core 通过 channel 向外发送事件（text_delta / tool_call / approval_required / done），由调用方（Tauri Command 或 napi 函数）决定如何消费。

3. **审批回调接口**：Core 中的 Agent 不自己弹窗，而是通过 trait `ApprovalHandler` 让调用方注入审批逻辑。CLI 用 stdin 输入，Desktop 用模态弹窗。

   ```rust
   #[async_trait]
   pub trait ApprovalHandler: Send + Sync {
       async fn approve(&self, tool: &str, params: &str) -> bool;
   }
   ```

4. **流式文本**：LLM 返回的 text delta 通过 Event 推送到前端，前端用 `requestAnimationFrame` 批量更新 DOM，避免卡顿。

5. **取消生成**：前端发送 `invoke('stop_generation')` → Tauri 层设置 `CancellationToken` → Core 的 Agent 循环检测并退出。CLI 同理，Ctrl+C 触发取消。

6. **多会话**：Core 提供 `AgentSession` 抽象，每个会话独立的 Agent 实例。会话列表存储在 SQLite 中。

7. **安全**：`bash` 工具的危险命令拦截保留在 Core 中，所有端共享同一套安全策略。文件操作可配置工作目录白名单。

8. **CLI 和 Desktop 同步发布**：Core 发版 → napi 绑定更新 → CLI 升级依赖；Core 发版 → Desktop Cargo.toml 更新 path 依赖。两个端各自独立发布，互不阻塞。
