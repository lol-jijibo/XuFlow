import type { MemoryHit, VectorIndex, VectorIndexItem } from "./types.js";
import { createLocalEmbeddingProvider, type EmbeddingProvider } from "./embeddingProvider.js";

export interface VectorIndexOptions {
  url: string;
  collection: string;
  vectorSize?: number;
  embeddingProvider?: EmbeddingProvider;
}

/**
 * 定义 Qdrant 向量索引的真实客户端，负责集合初始化和消息索引写入。
 * 写入时生成稳定向量并保存 payload，检索时从 Qdrant 取回内容再按文本相关度排序。
 */
export function createVectorIndex(options: VectorIndexOptions): VectorIndex {
  const embeddingProvider =
    options.embeddingProvider ??
    createLocalEmbeddingProvider(options.vectorSize ?? 64);
  const vectorSize = options.vectorSize ?? embeddingProvider.size;
  const baseUrl = options.url.replace(/\/$/, "");
  const ready = ensureCollection(baseUrl, options.collection, vectorSize);

  async function upsert(items: VectorIndexItem[]) {
    await ready;
    const valid = items.filter((item) => item.content?.trim());
    if (valid.length === 0) return;
    const points = await Promise.all(
      valid.map(async (item) => ({
        id: stablePointId(item.sessionId, item.messageId),
        vector: await embeddingProvider.embed(item.content),
        payload: item,
      }))
    );
    await qdrantFetch(
      baseUrl,
      `/collections/${options.collection}/points?wait=true`,
      {
        method: "PUT",
        body: JSON.stringify({ points }),
      }
    );
  }

  async function search(query: string, limit = 5): Promise<MemoryHit[]> {
    await ready;
    const response = await qdrantFetch(
      baseUrl,
      `/collections/${options.collection}/points/scroll`,
      {
        method: "POST",
        body: JSON.stringify({
          limit: Math.max(limit * 5, 10),
          with_payload: true,
          with_vector: false,
        }),
      }
    );
    const body = (await response.json()) as {
      result?: { points?: { payload?: VectorIndexItem }[] };
    };
    return (body.result?.points ?? [])
      .map((point) => toHit(point.payload, query))
      .filter((hit): hit is MemoryHit => hit !== null && hit.score > 0)
      .sort((a, b) => b.score - a.score)
      .slice(0, limit);
  }

  return { upsert, search };
}

async function ensureCollection(
  baseUrl: string,
  collection: string,
  vectorSize: number
) {
  const existing = await fetch(`${baseUrl}/collections/${collection}`);
  if (existing.ok) return;
  await qdrantFetch(baseUrl, `/collections/${collection}`, {
    method: "PUT",
    body: JSON.stringify({
      vectors: {
        size: vectorSize,
        distance: "Cosine",
      },
    }),
  });
}

async function qdrantFetch(
  baseUrl: string,
  path: string,
  init: RequestInit = {}
) {
  const response = await fetch(`${baseUrl}${path}`, {
    ...init,
    headers: {
      "content-type": "application/json",
      ...init.headers,
    },
  });
  if (!response.ok) {
    throw new Error(`Qdrant request failed: ${response.status} ${await response.text()}`);
  }
  return response;
}

function toHit(item: VectorIndexItem | undefined, query: string): MemoryHit | null {
  if (!item) return null;
  return {
    sessionId: item.sessionId,
    messageId: item.messageId,
    content: item.content,
    role: item.role,
    metadata: item.metadata,
    score: score(query, item.content),
  };
}

function score(query: string, content: string): number {
  const normalizedQuery = query.toLowerCase().trim();
  const normalizedContent = content.toLowerCase();
  if (!normalizedQuery || !normalizedContent) return 0;
  return normalizedQuery
    .split(/\s+/)
    .filter((token) => normalizedContent.includes(token)).length;
}

function stablePointId(sessionId: string, messageId: string): number {
  const source = `${sessionId}:${messageId}`;
  let hash = 2166136261;
  for (let i = 0; i < source.length; i++) {
    hash ^= source.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}
