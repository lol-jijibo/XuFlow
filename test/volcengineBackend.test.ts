import assert from "node:assert/strict";
import { createBackend } from "../src/backends/index.js";
import { VolcEngineChatBackend } from "../src/backends/volcengine.js";

const backend = createBackend({
  LLM_PROVIDER: "volcengine",
  VOLCENGINE_API_KEY: "test-key",
  VOLCENGINE_MODEL: "doubao-test-model",
  VOLCENGINE_BASE_URL: "https://ark.example.test/api/v3",
});

assert.equal(backend instanceof VolcEngineChatBackend, true);
assert.equal(backend.model, "doubao-test-model");

const selectedBackend = createBackend(
  {
    LLM_PROVIDER: "deepseek",
    VOLCENGINE_API_KEY: "test-key",
    VOLCENGINE_BASE_URL: "https://ark.example.test/api/v3",
  },
  { provider: "volcengine", model: "doubao-selected-model" }
);

assert.equal(selectedBackend instanceof VolcEngineChatBackend, true);
assert.equal(selectedBackend.model, "doubao-selected-model");
