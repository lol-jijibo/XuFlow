import assert from "node:assert/strict";
import { createVectorIndex } from "../src/memory/vectorIndex.js";
import type { EmbeddingProvider } from "../src/memory/embeddingProvider.js";

const calls: string[] = [];
const embeddingProvider: EmbeddingProvider = {
  size: 4,
  async embed(text: string) {
    calls.push(text);
    return [1, 0, 0, 0];
  },
};

const requests: { url: string; body: any }[] = [];
const originalFetch = globalThis.fetch;
globalThis.fetch = (async (input: RequestInfo | URL, init?: RequestInit) => {
  const url = String(input);
  if (url.endsWith("/collections/test_memory") && !init?.method) {
    return new Response(JSON.stringify({ result: {} }), { status: 404 });
  }
  requests.push({
    url,
    body: init?.body ? JSON.parse(String(init.body)) : undefined,
  });
  return new Response(JSON.stringify({ result: { points: [] } }), {
    status: 200,
    headers: { "content-type": "application/json" },
  });
}) as typeof fetch;

try {
  const index = createVectorIndex({
    url: "http://qdrant.test",
    collection: "test_memory",
    embeddingProvider,
  });
  await index.upsert([
    {
      sessionId: "session-a",
      messageId: "message-a",
      content: "hello embedding",
      role: "user",
      metadata: {},
    },
  ]);

  assert.deepEqual(calls, ["hello embedding"]);
  const upsert = requests.find((request) => request.url.includes("/points?wait=true"));
  assert.deepEqual(upsert?.body.points[0].vector, [1, 0, 0, 0]);
} finally {
  globalThis.fetch = originalFetch;
}
