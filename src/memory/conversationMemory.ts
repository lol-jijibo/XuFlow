import type { LlmMessage } from "../types.js";
import type {
  ConversationMemory,
  MemoryHit,
  MemorySessionSnapshot,
  ToolCallEntry,
  UIMessage,
} from "./types.js";
import { createSessionStore } from "./sessionStore.js";
import { createVectorIndex } from "./vectorIndex.js";
import { createEmbeddingProvider } from "./embeddingProvider.js";

export interface ConversationMemoryOptions {
  workspacePath: string;
  databaseUrl?: string;
  qdrantUrl?: string;
  qdrantCollection?: string;
}

/**
 * 编排长对话记忆链路，负责把 PostgreSQL 主存储和 Qdrant 索引串起来。
 * 启动时恢复最近会话，写入时先保存完整消息，再同步写入可检索索引。
 */
export function createConversationMemory(
  options: ConversationMemoryOptions
): ConversationMemory {
  const store = createSessionStore({
    connectionString:
      options.databaseUrl ??
      process.env.DATABASE_URL ??
      "postgres://xuflow:xuflow@localhost:15432/xuflow",
  });
  const index = createVectorIndex({
    url: options.qdrantUrl ?? process.env.QDRANT_URL ?? "http://localhost:6333",
    collection:
      options.qdrantCollection ??
      process.env.QDRANT_COLLECTION ??
      "xuflow_memory",
    embeddingProvider: createEmbeddingProvider(),
  });

  let session: MemorySessionSnapshot = {
    id: "",
    workspacePath: options.workspacePath,
    backend: "postgresql",
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    uiMessages: [],
    llmMessages: [],
  };

  const ready = (async () => {
    session =
      (await store.getLatestSession(options.workspacePath)) ??
      (await store.createSession(options.workspacePath));
  })();

  async function indexMessage(message: UIMessage) {
    if (message.role === "system") return;
    if (!message.content?.trim()) return;
    await index.upsert([
      {
        sessionId: session.id,
        messageId: message.id,
        content: message.content,
        role: message.role,
        metadata: { workspacePath: options.workspacePath },
      },
    ]);
  }

  return {
    get sessionId() {
      return session.id;
    },
    workspacePath: options.workspacePath,
    get uiMessages() {
      return session.uiMessages;
    },
    get llmMessages() {
      return session.llmMessages;
    },
    ready,
    async recall(query: string, limit = 5) {
      await ready;
      return (await index.search(query, limit)).map(formatHit).join("\n");
    },
    async appendUserMessage(message: UIMessage, llmMessage: LlmMessage) {
      await ready;
      session.uiMessages.push(message);
      session.llmMessages.push(llmMessage);
      await store.appendMessage(session.id, message);
      await store.appendLlmMessage(session.id, llmMessage);
      await indexMessage(message);
    },
    async appendAssistantMessage(message: UIMessage, llmMessage: LlmMessage) {
      await ready;
      session.uiMessages.push(message);
      session.llmMessages.push(llmMessage);
      await store.appendMessage(session.id, message);
      await store.appendLlmMessage(session.id, llmMessage);
      await indexMessage(message);
    },
    async appendToolMessage(llmMessage: LlmMessage) {
      await ready;
      session.llmMessages.push(llmMessage);
      await store.appendLlmMessage(session.id, llmMessage);
    },
    async updateToolResult(
      toolCallId: string,
      result: string,
      status: ToolCallEntry["status"]
    ) {
      await ready;
      await store.updateToolResult(session.id, toolCallId, result, status);
    },
    async startNewSession() {
      await ready;
      session = await store.createSession(options.workspacePath);
    },
    async close() {
      await store.close();
    },
  };
}

function formatHit(hit: MemoryHit): string {
  return `[${hit.score.toFixed(2)}] ${hit.role}: ${hit.content}`;
}
