# Xuflow

AI 终端编程助手 — 基于 Ink（React for terminal）构建的 TUI 应用，通过 OpenAI 兼容 SDK 接入多个 LLM 后端，支持长对话记忆持久化与语义搜索。

## 项目目录

```text
Xuflow/
├── .env.example                              # 环境变量模板
├── .gitignore                                # Git 忽略规则
├── README.md                                 # 本文件
├── docker-compose.yml                        # PostgreSQL 服务定义
├── package.json                              # 项目元信息与依赖
├── package-lock.json                         # 依赖锁定文件
├── tsconfig.json                             # TypeScript 编译配置
│
├── bin/
│   └── xuflow.js                             # CLI 启动器（npm link 后可全局调用）
│
├── doc/
│   ├── 2026-06-11-memory-postgres-qdrant-volcengine.md  # 长对话记忆接入说明
│   └── doubao-embedding-vision-2026-06-12.md            # 火山多模态 Embedding 接入说明
│
├── src/
│   ├── loop.ts                               # 入口：渲染 Ink TUI，初始化后端与记忆
│   ├── types.ts                              # 共享类型 + LLMBackend 接口 + 消息清洗
│   ├── tools.ts                              # 工具注册 / 执行 / 安全策略
│   ├── modelConfig.ts                        # 多供应商模型配置与路由
│   ├── modelPrefs.ts                         # 模型偏好持久化（.xuflow_prefs.json）
│   │
│   ├── backends/
│   │   ├── index.ts                          # 后端工厂（按 LLM_PROVIDER 切换）
│   │   ├── deepseek.ts                       # DeepSeek OpenAI 兼容后端
│   │   └── volcengine.ts                     # 火山方舟 OpenAI 兼容后端
│   │
│   ├── memory/
│   │   ├── index.ts                          # 记忆模块聚合导出
│   │   ├── types.ts                          # 记忆层类型定义
│   │   ├── conversationMemory.ts             # 对话记忆编排（PG + Qdrant + Embedding）
│   │   ├── sessionStore.ts                   # PostgreSQL 会话存储
│   │   ├── vectorIndex.ts                    # Qdrant 向量索引
│   │   ├── embeddingProvider.ts              # 火山引擎 / 本地确定性 Embedding
│   │   └── volcengine.ts                     # 火山引擎记忆能力描述对象
│   │
│   └── ui/
│       ├── app.tsx                           # Ink TUI 主组件（Header/MessageList/InputBar/弹出面板）
│       ├── useAgent.ts                       # Agent 对话循环 Hook
│       ├── inputCursor.ts                    # IME 光标定位工具
│       └── workspaceStatus.ts                # 工作区名称工具
│
└── test/
    ├── workspaceStatus.test.ts               # 工作区名称测试
    ├── inputCursorAnsi.test.ts               # 光标 ANSI 序列测试
    ├── inputCursorLayout.test.ts             # 光标布局测试
    ├── modelConfig.test.ts                   # 模型配置路由测试
    ├── embeddingProvider.test.ts             # Embedding 提供者测试
    ├── volcengineBackend.test.ts             # 火山引擎聊天后端测试
    ├── volcengineMemory.test.ts              # 火山引擎记忆链路测试
    ├── volcengineMultimodalEmbedding.test.ts # 多模态 Embedding 测试
    └── realMemoryServices.test.ts            # 真实记忆服务集成测试
```

## 核心架构

```text
┌─────────────────────────────────────────────┐
│  src/ui/app.tsx          Ink TUI 组件树      │
│  src/ui/useAgent.ts      对话循环状态管理     │
├─────────────────────────────────────────────┤
│  src/tools.ts            工具系统 + 安全策略  │
│  src/backends/           LLM 后端适配层       │
│  src/memory/             长对话记忆三层存储   │
├─────────────────────────────────────────────┤
│  src/types.ts            统一类型 + 接口契约  │
│  src/modelConfig.ts      多供应商模型路由     │
└─────────────────────────────────────────────┘
```

## 快速开始

```bash
# 1. 启动 PostgreSQL
docker compose up -d

# 2. 安装依赖
npm install

# 3. 配置环境变量
cp .env.example .env
# 编辑 .env，填入 DEEPSEEK_API_KEY 或 VOLCENGINE_API_KEY

# 4. 启动 TUI
npm run dev
```

## 功能特性

| 特性 | 说明 |
|------|------|
| **Plan / Act 双模式** | Plan 只读分析 → Act 执行修改，Ctrl+T / `/mode` 切换 |
| **多 LLM 后端** | DeepSeek + 火山方舟（10+ 模型端点），`/model` 面板切换 |
| **流式对话** | 实时文本渲染 + token 计数，最大 15 轮工具调用循环 |
| **5 个内置工具** | read_file / write_file / list_dir / grep / bash |
| **安全策略** | 敏感文件保护 + 危险命令检测 + 用户审批确认 |
| **长对话记忆** | PostgreSQL（真相源）+ Qdrant（向量索引）+ 火山 Embedding |
| **斜杠命令** | `/mode` `/model` `/new` `/history` `/recall <关键词>` |
| **IME 适配** | 自定义光标定位，支持中文输入法候选窗正确显示 |
| **偏好持久化** | 自动记住上次使用的模型，重启恢复 |

## 环境变量

| 变量 | 说明 | 示例 |
|------|------|------|
| `LLM_PROVIDER` | LLM 供应商（deepseek / volcengine） | `deepseek` |
| `DEEPSEEK_API_KEY` | DeepSeek API Key | `sk-...` |
| `DEEPSEEK_MODEL` | DeepSeek 模型名 | `deepseek-v4-pro` |
| `VOLCENGINE_API_KEY` | 火山方舟 API Key | — |
| `VOLCENGINE_MODEL` | 火山方舟默认模型 | `doubao-pro` |
| `VOLCENGINE_MODELS` | 火山方舟可用模型列表（逗号分隔） | `deepseek-v4-pro-260425,...` |
| `VOLCENGINE_BASE_URL` | 火山方舟网关地址 | `https://ark.cn-beijing.volces.com/api/v3` |
| `VOLCENGINE_EMBEDDING_MODEL` | 火山 Embedding 模型 | `ep-...` |
| `VOLCENGINE_EMBEDDING_MODE` | Embedding 模式（text / multimodal） | `multimodal` |
| `VOLCENGINE_EMBEDDING_SIZE` | Embedding 向量维度 | `2048` |
| `DATABASE_URL` | PostgreSQL 连接串 | `postgres://xuflow:xuflow@localhost:15432/xuflow` |
| `QDRANT_URL` | Qdrant 服务地址 | `http://localhost:6333` |
| `QDRANT_COLLECTION` | Qdrant 集合名 | `xuflow_memory` |

## 扩展指南

- **新增 LLM 后端**：实现 `LLMBackend` 接口（`chat()` 流式生成器），在 `backends/index.ts` 注册
- **新增工具**：在 `BUILTIN_TOOLS` 添加 `ToolDef`，在 `executeTool` 添加 `case`，必要时加入 `DANGEROUS_TOOLS`
- **新增斜杠命令**：在 `SLASH_COMMANDS` 注册，在 `handleSubmit` 添加分支

## 技术栈

| 层级 | 技术 |
|------|------|
| 运行时 | Node.js + TypeScript |
| 终端 UI | Ink（React for terminal） |
| LLM SDK | OpenAI 兼容 SDK |
| 聊天后端 | DeepSeek API / 火山方舟 |
| 主存储 | PostgreSQL 16 |
| 向量索引 | Qdrant |
| Embedding | 火山引擎 Doubao-embedding-vision（2048 维） |
| 容器化 | Docker Compose |
