# Doubao-embedding-vision 接入说明

## 背景

本次接入把 Xuflow 长对话记忆里的向量生成能力，从普通文本 embedding 切到火山方舟的 `Doubao-embedding-vision` endpoint。
PostgreSQL 仍然保存完整原始会话，Qdrant 仍然保存向量索引，火山方舟只负责把文本或图片输入转换成向量。

## 功能

1. 使用 `VOLCENGINE_EMBEDDING_MODEL=ep-20260612171932-jnzkr` 调用当前已开通的方舟 endpoint。
2. 使用 `VOLCENGINE_EMBEDDING_MODE=multimodal` 切换到 `/embeddings/multimodal` 接口。
3. 普通对话文本会自动包装为 `{ "type": "text" }` 后发送给火山方舟。
4. embedding provider 已支持传入 `{ "type": "image_url" }`，后续接图片附件时可以复用同一入口。
5. 真实接口返回向量维度为 2048，Qdrant 的 `xuflow_memory` collection 已按 2048 维重建。

## 业务逻辑

用户在 Xuflow 中输入问题后，消息先写入 PostgreSQL 的 `memory_messages` 表，作为完整历史的真实来源。
随后可检索的 user/assistant 消息会交给 embedding provider；provider 根据环境变量选择火山多模态接口，把文本包装成多模态 input 后生成 2048 维向量。
向量和轻量 payload 会写入 Qdrant 的 `xuflow_memory` collection，用于后续语义召回；Qdrant 不保存完整会话事实。

## 环境变量

```env
VOLCENGINE_BASE_URL=https://ark.cn-beijing.volces.com/api/v3
VOLCENGINE_EMBEDDING_MODEL=ep-20260612171932-jnzkr
VOLCENGINE_EMBEDDING_MODE=multimodal
VOLCENGINE_EMBEDDING_SIZE=2048
QDRANT_COLLECTION=xuflow_memory
```

`VOLCENGINE_API_KEY` 只写在本地 `.env`，不要写入文档或提交记录。

## 验证结果

已用真实火山接口测得 `Doubao-embedding-vision` 返回 2048 维向量。
已删除旧的 4096 维 Qdrant collection，并由程序自动重建为 2048 维。
已通过真实写入验证，PostgreSQL 保存原始消息，Qdrant 保存对应向量点。

## 验证命令

```powershell
npm run build
npx tsx test\embeddingProvider.test.ts
npx tsx test\volcengineMultimodalEmbedding.test.ts
npx tsx test\realMemoryServices.test.ts
```
