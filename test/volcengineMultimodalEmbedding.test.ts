import assert from "node:assert/strict";
import { createVolcEngineEmbeddingProvider } from "../src/memory/embeddingProvider.js";

const originalFetch = globalThis.fetch;
const requests: { url: string; body: any; authorization?: string }[] = [];
const responseBodies = [
  { data: { embedding: [0.1, 0.2, 0.3] } },
  { data: { embedding: [0.4, 0.5, 0.6] } },
];

globalThis.fetch = (async (input: RequestInfo | URL, init?: RequestInit) => {
  requests.push({
    url: String(input),
    body: JSON.parse(String(init?.body ?? "{}")),
    authorization: (init?.headers as Record<string, string>)?.Authorization,
  });
  return new Response(
    JSON.stringify(responseBodies.shift()),
    { status: 200, headers: { "content-type": "application/json" } }
  );
}) as typeof fetch;

try {
  const provider = createVolcEngineEmbeddingProvider({
    apiKey: "ark-test",
    model: "ep-test",
    mode: "multimodal",
    size: 3,
  });
  const embedding = await provider.embed("天很蓝，海很深");
  const imageEmbedding = await provider.embed([
    { type: "text", text: "天很蓝，海很深" },
    {
      type: "image_url",
      image_url: {
        url: "https://ark-project.tos-cn-beijing.volces.com/images/view.jpeg",
      },
    },
  ]);

  assert.deepEqual(embedding, [0.1, 0.2, 0.3]);
  assert.deepEqual(imageEmbedding, [0.4, 0.5, 0.6]);
  assert.equal(
    requests[0].url,
    "https://ark.cn-beijing.volces.com/api/v3/embeddings/multimodal"
  );
  assert.equal(requests[0].authorization, "Bearer ark-test");
  assert.deepEqual(requests[0].body, {
    model: "ep-test",
    input: [{ type: "text", text: "天很蓝，海很深" }],
  });
  assert.deepEqual(requests[1].body, {
    model: "ep-test",
    input: [
      { type: "text", text: "天很蓝，海很深" },
      {
        type: "image_url",
        image_url: {
          url: "https://ark-project.tos-cn-beijing.volces.com/images/view.jpeg",
        },
      },
    ],
  });
} finally {
  globalThis.fetch = originalFetch;
}
