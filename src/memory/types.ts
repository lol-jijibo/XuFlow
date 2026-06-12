import type { LlmMessage } from "../types.js";

export interface ToolCallEntry {
  id: string;
  name: string;
  args: Record<string, unknown>;
  result?: string;
  status: "running" | "done" | "denied" | "error";
}

export interface UIMessage {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  toolCalls?: ToolCallEntry[];
  usage?: { inputTokens: number; outputTokens: number };
}

export interface MemoryHit {
  sessionId: string;
  messageId: string;
  content: string;
  role: "user" | "assistant";
  score: number;
  metadata: Record<string, unknown>;
}

export interface ConversationMemory {
  sessionId: string;
  workspacePath: string;
  uiMessages: UIMessage[];
  llmMessages: LlmMessage[];
  ready: Promise<void>;
  recall(query: string, limit?: number): Promise<string>;
  appendUserMessage(message: UIMessage, llmMessage: LlmMessage): Promise<void>;
  appendAssistantMessage(message: UIMessage, llmMessage: LlmMessage): Promise<void>;
  appendToolMessage(llmMessage: LlmMessage): Promise<void>;
  updateToolResult(
    toolCallId: string,
    result: string,
    status: ToolCallEntry["status"]
  ): Promise<void>;
  startNewSession(): Promise<void>;
  close(): Promise<void>;
}

export interface MemorySessionSnapshot {
  id: string;
  workspacePath: string;
  backend: "postgresql";
  createdAt: string;
  updatedAt: string;
  uiMessages: UIMessage[];
  llmMessages: LlmMessage[];
}

export interface SessionStore {
  createSession(workspacePath: string): Promise<MemorySessionSnapshot>;
  getSession(sessionId: string): Promise<MemorySessionSnapshot | undefined>;
  getLatestSession(workspacePath: string): Promise<MemorySessionSnapshot | undefined>;
  appendMessage(sessionId: string, message: UIMessage): Promise<void>;
  appendLlmMessage(sessionId: string, message: LlmMessage): Promise<void>;
  updateToolResult(
    sessionId: string,
    toolCallId: string,
    result: string,
    status: ToolCallEntry["status"]
  ): Promise<void>;
  close(): Promise<void>;
}

export interface VectorIndexItem {
  sessionId: string;
  messageId: string;
  content: string;
  role: "user" | "assistant";
  metadata: Record<string, unknown>;
}

export interface VectorIndex {
  upsert(items: VectorIndexItem[]): Promise<void>;
  search(query: string, limit?: number): Promise<MemoryHit[]>;
}
