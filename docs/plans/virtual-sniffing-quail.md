# Mirro — AI 问题解决引擎 · 实施计划

## Context

从零构建一个**前后端分离**的 AI 问题解决平台，用于提升简历亮点。

- **前端**：Vue 3 + TypeScript + Tailwind CSS（Vite 构建）
- **后端**：Python（FastAPI）
- **目标**：MVP 初级版，核心链路跑通

---

## 项目结构

```
Mirro/
├── frontend/                 # Vue 3 + TS + Tailwind
│   ├── src/
│   │   ├── main.ts
│   │   ├── App.vue
│   │   ├── router/index.ts
│   │   ├── stores/question.ts
│   │   ├── api/client.ts
│   │   ├── types/index.ts
│   │   ├── views/
│   │   │   ├── HomeView.vue        # 首页+提问入口
│   │   │   ├── QuestionView.vue    # 答案详情页
│   │   │   ├── HistoryView.vue     # 历史问题列表
│   │   │   └── DashboardView.vue   # 问题统计看板
│   │   ├── components/
│   │   │   ├── QuestionInput.vue       # 问题输入框
│   │   │   ├── StreamingAnswer.vue     # SSE流式答案渲染
│   │   │   ├── FlowchartViewer.vue     # Mermaid流程图
│   │   │   ├── SolutionSteps.vue       # 分步执行计划
│   │   │   ├── SourceList.vue          # 搜索来源引用
│   │   │   ├── NavBar.vue              # 导航栏
│   │   │   └── SkeletonLoader.vue      # 骨架屏加载
│   │   └── assets/main.css
│   ├── index.html
│   ├── vite.config.ts
│   ├── tailwind.config.js
│   ├── tsconfig.json
│   └── package.json
│
├── backend/                  # Python FastAPI
│   ├── app/
│   │   ├── main.py          # FastAPI 入口, CORS配置
│   │   ├── api/
│   │   │   ├── questions.py # 问题相关路由
│   │   │   └── search.py    # 搜索路由
│   │   ├── services/
│   │   │   ├── ai_service.py      # LLM调用 (OpenAI/Claude)
│   │   │   ├── search_service.py  # 搜索聚合 (Tavily)
│   │   │   └── flowchart_service.py # 流程图生成
│   │   ├── models/
│   │   │   ├── database.py  # SQLite 连接 + 建表
│   │   │   └── schemas.py   # Pydantic 模型
│   │   └── core/
│   │       └── config.py    # 环境变量配置
│   ├── requirements.txt
│   └── .env.example
│
└── README.md
```

---

## 技术选型详情

### 前端
| 类别 | 选择 | 理由 |
|------|------|------|
| 框架 | Vue 3 + Composition API + TypeScript | 用户指定 |
| 构建 | Vite | 快、Vue 生态标配 |
| 样式 | Tailwind CSS | 用户指定 |
| 路由 | Vue Router 4 | 标配 |
| 状态管理 | Pinia | Vue 3 官方推荐 |
| HTTP | fetch + SSE (EventSource) | 原生支持流式 |
| Markdown | markdown-it + highlight.js | 轻量 |
| 流程图 | Mermaid.js | 前端直接渲染，无需后端 |
| 图表 | ECharts (可选，Dashboard用) | 国内常用 |

### 后端
| 类别 | 选择 | 理由 |
|------|------|------|
| 框架 | FastAPI | 异步、SSE原生支持、自动文档 |
| LLM | OpenAI API (兼容 Claude 也可用) | 通过环境变量切换 |
| 搜索 | Tavily Search API | 专为 AI Agent 设计，支持中文 |
| 数据库 | SQLite + aiosqlite | MVP 零配置，后续可换 PostgreSQL |
| 验证 | Pydantic v2 | FastAPI 内置 |
| 服务 | uvicorn | ASGI 服务器 |

---

## MVP 功能范围

### Phase 1：核心链路（本次实现）
1. **提问页** — 输入问题 → 提交 → SSE 流式输出答案
2. **答案页** — 结构化展示：
   - Markdown 正文（流式渲染）
   - Mermaid 流程图（展示解决路径）
   - 分步执行计划
   - 搜索来源引用
3. **历史列表** — 查看所有提问记录
4. **统计看板** — 问题分类饼图 + 提问趋势

### API 设计

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/questions` | 提交问题，返回 SSE 流式答案 |
| `GET` | `/api/questions` | 获取问题历史列表 |
| `GET` | `/api/questions/{id}` | 获取单个问题详情+答案 |
| `DELETE` | `/api/questions/{id}` | 删除问题 |
| `POST` | `/api/search` | 搜索相关案例资源 |

### 数据库表

```sql
questions:
  id TEXT PRIMARY KEY,
  content TEXT NOT NULL,
  category TEXT,
  created_at TEXT NOT NULL

answers:
  id TEXT PRIMARY KEY,
  question_id TEXT FK,
  content TEXT,           -- Markdown 答案
  flowchart_mermaid TEXT, -- Mermaid 语法
  steps JSON,             -- 分步计划 [{step, title, desc, duration}]
  sources JSON,           -- 引用来源 [{title, url, snippet}]
  created_at TEXT NOT NULL
```

---

## 代码规范

**所有代码必须添加中文业务注释**，注释原则：
- 每个文件头部：说明该文件在整体业务中的角色
- 每个函数/方法：说明它解决什么业务问题
- 关键逻辑处：解释为什么这么写，业务含义是什么
- 类型定义处：字段说明业务含义（例：`flowchart_mermaid` → "AI 生成的 Mermaid 流程图语法"）
- 数据库字段：comment 说明业务用途
- 不写废话注释（如 `// 声明变量`），只写业务含义

---

## 实现步骤（按顺序）

### Step 1: 项目脚手架
- 创建 `frontend/` (Vite + Vue 3 + TS + Tailwind)
- 创建 `backend/` (Python 虚拟环境 + FastAPI + uvicorn)
- 配置 CORS、环境变量

### Step 2: 后端核心 API
- 数据库初始化 + SQLite 建表
- `POST /api/questions` — 核心接口：接收问题 → 调 LLM 生成答案+流程图+步骤+搜索 → SSE 流式返回
- `GET /api/questions` — 历史列表
- `GET /api/questions/{id}` — 详情
- `POST /api/search` — 外部搜索

### Step 3: 前端核心页面
- `HomeView` — 提问入口 + 大标题
- `QuestionInput` — 输入框 + 提交按钮
- `QuestionView` — 答案展示页（路由 `/question/:id`）
- `StreamingAnswer` — SSE 逐字渲染 Markdown
- `FlowchartViewer` — Mermaid 渲染流程图
- `SolutionSteps` — 步骤卡片列表
- `SourceList` — 来源链接

### Step 4: 前端辅助页面
- `HistoryView` — 历史问题列表
- `DashboardView` — 简单统计图表

### Step 5: 前后端联调 + 装点
- 骨架屏、过渡动画
- Dark Mode 支持
- 错误处理/空状态

---

## 验证方式

1. 启动后端 `uvicorn app.main:app --reload`
2. 启动前端 `npm run dev`
3. 浏览器打开前端 → 输入问题 → 提交
4. 观察 SSE 流式输出 → Flowchart 渲染 → 步骤展示
5. 检查历史列表 → 检查统计看板
6. API 文档 `http://localhost:8000/docs` 自测

---

## 待定问题
- LLM API Key：需要用户提供 OpenAI 或 Anthropic API Key
- Tavily API Key：需要用户注册（免费 1000 次/月）
- 是否先用 Mock 数据跑通前端，再加真实 API？
