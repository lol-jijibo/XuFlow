import assert from "node:assert/strict";
import { Client } from "pg";

const databaseUrl =
  process.env.DATABASE_URL ??
  "postgres://xuflow:xuflow@localhost:15432/xuflow";
const qdrantUrl = process.env.QDRANT_URL ?? "http://localhost:6333";

const pg = new Client({ connectionString: databaseUrl });
await pg.connect();

try {
  const session = await pg.query(
    "select id, workspace_path from memory_sessions order by updated_at desc limit 1"
  );
  assert.equal(session.rowCount, 1);
  assert.equal(typeof session.rows[0].id, "string");

  const messages = await pg.query(
    "select role, content from memory_messages where session_id = $1 order by created_at asc",
    [session.rows[0].id]
  );
  assert.equal(messages.rowCount > 0, true);
  assert.equal(typeof messages.rows[0].content, "string");
} finally {
  await pg.end();
}

const collections = await fetch(`${qdrantUrl}/collections/xuflow_memory`);
assert.equal(collections.ok, true);

const scroll = await fetch(
  `${qdrantUrl}/collections/xuflow_memory/points/scroll`,
  {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ limit: 1, with_payload: true, with_vector: false }),
  }
);
assert.equal(scroll.ok, true);
const body = (await scroll.json()) as { result?: { points?: unknown[] } };
assert.equal((body.result?.points?.length ?? 0) > 0, true);
