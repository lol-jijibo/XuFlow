# 长对话记忆与真实服务接入说明

## 背景

Xuflow 原本只在运行时内存中保存对话，进程退出后历史会丢失。
这次实现把长对话记忆拆成三层：PostgreSQL 保存完整会话，Qdrant 保存检索索引，火山引擎提供 chat 和 embedding API 能力。

## 功能

1. 使用 Docker Compose 启动 PostgreSQL。
2. 复用本机 Qdrant 服务，默认地址为 `http://localhost:6333`。
3. 启动 CLI 时自动连接 PostgreSQL，并创建 `memory_sessions` 和 `memory_messages` 表。
4. 用户消息和助手回复会写入 PostgreSQL，作为完整历史的真相源。
5. 可检索消息会同步写入 Qdrant 的 `xuflow_memory` collection。
6. 设置 `LLM_PROVIDER=volcengine` 后，聊天模型切换到火山方舟 OpenAI 兼容接口。
7. 配置 `VOLCENGINE_EMBEDDING_MODEL` 后，Qdrant 写入会使用火山引擎 embedding；未配置时降级为本地确定性向量。

## 业务逻辑

CLI 启动时会等待记忆服务初始化完成，优先恢复当前工作区最近一次会话，没有历史时创建新会话。
用户发送消息后，系统先把 UI 消息和 LLM 消息写入 PostgreSQL，再把 user/assistant 内容写入 Qdrant。
Qdrant 只保存检索 payload 和向量，不承担完整历史回放；完整会话始终以 PostgreSQL 为准。

## 启动方式

```powershell
docker compose up -d
npm run dev
```

## 环境变量

```env
LLM_PROVIDER=deepseek
DEEPSEEK_API_KEY=sk-...
DEEPSEEK_MODEL=deepseek-v4-pro

VOLCENGINE_API_KEY=
VOLCENGINE_MODEL=doubao-pro
VOLCENGINE_BASE_URL=https://ark.cn-beijing.volces.com/api/v3
VOLCENGINE_EMBEDDING_MODEL=ep-20260612171932-jnzkr
VOLCENGINE_EMBEDDING_MODE=multimodal
VOLCENGINE_EMBEDDING_SIZE=2048

DATABASE_URL=postgres://xuflow:xuflow@localhost:15432/xuflow
QDRANT_URL=http://localhost:6333
QDRANT_COLLECTION=xuflow_memory
```

## 火山引擎启用方式

火山引擎不是本地进程，不需要 Docker 启动。
它是云 API，只有在配置 API Key 和模型后才会被调用。

聊天模型切换到火山引擎：

```env
LLM_PROVIDER=volcengine
VOLCENGINE_API_KEY=你的火山方舟 API Key
VOLCENGINE_MODEL=你的火山方舟 chat endpoint 或模型名
```

Qdrant 向量改用火山 embedding：

```env
VOLCENGINE_API_KEY=你的火山方舟 API Key
VOLCENGINE_EMBEDDING_MODEL=ep-20260612171932-jnzkr
VOLCENGINE_EMBEDDING_MODE=multimodal
VOLCENGINE_EMBEDDING_SIZE=2048
```

当前接入的是 `Doubao-embedding-vision` 的方舟 endpoint，调用路径为 `/embeddings/multimodal`。
普通对话文本会包装为 `{ "type": "text" }` 后生成 2048 维向量；后续接入图片附件时，可以把图片 URL 作为 `{ "type": "image_url" }` 一起传入同一个 embedding provider。

## 查看数据

PostgreSQL：

- 数据库：`xuflow`
- schema：`public`
- 表：`memory_sessions`
- 表：`memory_messages`

Qdrant：

- collection：`xuflow_memory`

## 验证方式

```powershell
npm run build
npx tsx test\realMemoryServices.test.ts
npx tsx test\volcengineBackend.test.ts
npx tsx test\embeddingProvider.test.ts
npx tsx test\volcengineMultimodalEmbedding.test.ts
```
