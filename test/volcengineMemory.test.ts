import assert from "node:assert/strict";
import { createVolcEngineBackend } from "../src/memory/index.js";

const backend = createVolcEngineBackend({
  apiKey: "test-key",
  model: "doubao-pro",
  endpoint: "https://example.invalid",
});

const description = backend.describe();
assert.equal(description.provider, "volcengine");
assert.equal(description.model, "doubao-pro");
assert.equal(description.endpoint, "https://example.invalid");
