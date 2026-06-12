import { Client } from "pg";
import type { LlmMessage } from "../types.js";
import type {
  MemorySessionSnapshot,
  SessionStore,
  ToolCallEntry,
  UIMessage,
} from "./types.js";

export interface SessionStoreOptions {
  connectionString: string;
}

/**
 * 集中管理长对话主存储，负责连接 PostgreSQL 并维护会话表。
 * 启动时自动建表，写入时把 UI 消息和 LLM 消息分别保存到同一会话下。
 */
export function createSessionStore(options: SessionStoreOptions): SessionStore {
  const client = new Client({ connectionString: options.connectionString });
  const ready = connectAndMigrate(client);

  async function createSession(workspacePath: string) {
    await ready;
    const id = `session_${Date.now()}_${Math.random().toString(16).slice(2, 8)}`;
    const result = await client.query(
      `insert into memory_sessions (id, workspace_path)
       values ($1, $2)
       returning id, workspace_path, created_at, updated_at`,
      [id, workspacePath]
    );
    return toSession(result.rows[0], [], []);
  }

  async function getSession(sessionId: string) {
    await ready;
    const result = await client.query(
      `select id, workspace_path, created_at, updated_at
       from memory_sessions
       where id = $1`,
      [sessionId]
    );
    if (result.rowCount === 0) return undefined;
    return hydrateSession(result.rows[0]);
  }

  async function getLatestSession(workspacePath: string) {
    await ready;
    const result = await client.query(
      `select id, workspace_path, created_at, updated_at
       from memory_sessions
       where workspace_path = $1
       order by updated_at desc
       limit 1`,
      [workspacePath]
    );
    if (result.rowCount === 0) return undefined;
    return hydrateSession(result.rows[0]);
  }

  async function appendMessage(sessionId: string, message: UIMessage) {
    await ready;
    await client.query(
      `insert into memory_messages (session_id, message_id, channel, role, content, payload)
       values ($1, $2, 'ui', $3, $4, $5::jsonb)`,
      [sessionId, message.id, message.role, message.content, JSON.stringify(message)]
    );
    await touchSession(sessionId);
  }

  async function appendLlmMessage(sessionId: string, message: LlmMessage) {
    await ready;
    await client.query(
      `insert into memory_messages (session_id, message_id, channel, role, content, payload)
       values ($1, $2, 'llm', $3, $4, $5::jsonb)`,
      [
        sessionId,
        `llm_${Date.now()}_${Math.random().toString(16).slice(2, 8)}`,
        message.role,
        message.content ?? "",
        JSON.stringify(message),
      ]
    );
    await touchSession(sessionId);
  }

  async function updateToolResult(
    sessionId: string,
    toolCallId: string,
    result: string,
    status: ToolCallEntry["status"]
  ) {
    await ready;
    const rows = await client.query(
      `select id, payload
       from memory_messages
       where session_id = $1 and channel = 'ui'
       order by created_at desc`,
      [sessionId]
    );
    for (const row of rows.rows) {
      const payload = row.payload as UIMessage;
      const call = payload.toolCalls?.find((item) => item.id === toolCallId);
      if (!call) continue;
      call.result = result;
      call.status = status;
      await client.query(
        `update memory_messages
         set payload = $1::jsonb, content = $2
         where id = $3`,
        [JSON.stringify(payload), payload.content, row.id]
      );
      await touchSession(sessionId);
      return;
    }
  }

  async function hydrateSession(row: any): Promise<MemorySessionSnapshot> {
    const messages = await client.query(
      `select channel, payload
       from memory_messages
       where session_id = $1
       order by created_at asc, id asc`,
      [row.id]
    );
    const uiMessages = messages.rows
      .filter((message) => message.channel === "ui")
      .map((message) => message.payload as UIMessage);
    const llmMessages = messages.rows
      .filter((message) => message.channel === "llm")
      .map((message) => message.payload as LlmMessage);
    return toSession(row, uiMessages, llmMessages);
  }

  async function touchSession(sessionId: string) {
    await client.query(
      `update memory_sessions set updated_at = now() where id = $1`,
      [sessionId]
    );
  }

  return {
    createSession,
    getSession,
    getLatestSession,
    appendMessage,
    appendLlmMessage,
    updateToolResult,
    async close() {
      await ready;
      await client.end();
    },
  };
}

async function connectAndMigrate(client: Client) {
  await client.connect();
  await client.query(`
    create table if not exists memory_sessions (
      id text primary key,
      workspace_path text not null,
      created_at timestamptz not null default now(),
      updated_at timestamptz not null default now()
    );

    create table if not exists memory_messages (
      id bigserial primary key,
      session_id text not null references memory_sessions(id) on delete cascade,
      message_id text not null,
      channel text not null check (channel in ('ui', 'llm')),
      role text not null,
      content text not null default '',
      payload jsonb not null,
      created_at timestamptz not null default now()
    );

    create index if not exists memory_messages_session_idx
      on memory_messages(session_id, created_at);
  `);
}

function toSession(
  row: any,
  uiMessages: UIMessage[],
  llmMessages: LlmMessage[]
): MemorySessionSnapshot {
  return {
    id: row.id,
    workspacePath: row.workspace_path,
    backend: "postgresql",
    createdAt: new Date(row.created_at).toISOString(),
    updatedAt: new Date(row.updated_at).toISOString(),
    uiMessages,
    llmMessages,
  };
}
