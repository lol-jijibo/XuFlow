import assert from "node:assert/strict";
import { getDeepSeekModel, getModelOptions } from "../src/modelConfig.js";

assert.equal(getDeepSeekModel({ DEEPSEEK_MODEL: "deepseek-v4-pro" }), "deepseek-v4-pro");
assert.equal(getDeepSeekModel({ DEEPSEEK_MODEL: "  deepseek-v4-flash  " }), "deepseek-v4-flash");
assert.equal(getDeepSeekModel({ DEEPSEEK_MODEL: "" }), "deepseek-chat");
assert.equal(getDeepSeekModel({}), "deepseek-chat");

const options = getModelOptions({
  DEEPSEEK_MODEL: "deepseek-v4-pro",
  VOLCENGINE_MODEL: "doubao-test-model",
});

assert.deepEqual(
  options.map((option) => ({
    provider: option.provider,
    model: option.model,
    key: option.key,
  })),
  [
    { provider: "deepseek", model: "deepseek-v4-pro", key: "D" },
    { provider: "volcengine", model: "doubao-test-model", key: "V" },
  ]
);

const volcengineOptions = getModelOptions({
  DEEPSEEK_MODEL: "deepseek-v4-pro",
  VOLCENGINE_MODELS: "doubao-seed-2-0-code,doubao-seed-1-8, doubao-seed-2-0-lite ",
});

assert.deepEqual(
  volcengineOptions
    .filter((option) => option.provider === "volcengine")
    .map((option) => option.model),
  ["doubao-seed-2-0-code", "doubao-seed-1-8", "doubao-seed-2-0-lite"]
);

const defaultVolcengineModels = getModelOptions({})
  .filter((option) => option.provider === "volcengine")
  .map((option) => option.model);

assert.deepEqual(defaultVolcengineModels, [
  "deepseek-v4-flash-260425",
  "deepseek-v4-pro-260425",
  "doubao-seed-2-0-code-preview-260215",
  "doubao-seed-1-8-251228",
  "doubao-seed-2-0-lite-260428",
  "doubao-seed-2-0-mini-260428",
  "doubao-seed-2-0-pro-260215",
  "doubao-seed-character-251128",
  "glm-4-7-251222",
  "deepseek-v3-2-251201",
]);

assert.equal(
  getModelOptions({})
    .filter((option) => option.provider === "volcengine")
    .find((option) => option.model === "deepseek-v4-flash-260425")?.label,
  "DeepSeek-V4-flash"
);
