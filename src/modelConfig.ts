const DEFAULT_DEEPSEEK_MODEL = "deepseek-chat";
const DEFAULT_VOLCENGINE_MODEL = "doubao-pro";
const DEFAULT_VOLCENGINE_MODELS = [
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
];
const VOLCENGINE_MODEL_LABELS: Record<string, string> = {
  "deepseek-v4-flash-260425": "DeepSeek-V4-flash",
  "deepseek-v4-pro-260425": "DeepSeek-V4-pro",
  "doubao-seed-2-0-code-preview-260215": "Doubao-Seed-2.0-Code",
  "doubao-seed-1-8-251228": "Doubao-Seed-1.8",
  "doubao-seed-2-0-lite-260428": "Doubao-Seed-2.0-lite",
  "doubao-seed-2-0-mini-260428": "Doubao-Seed-2.0-mini",
  "doubao-seed-2-0-pro-260215": "Doubao-Seed-2.0-pro",
  "doubao-seed-character-251128": "Doubao-Seed-Character",
  "glm-4-7-251222": "GLM-4.7",
  "deepseek-v3-2-251201": "DeepSeek-V3.2",
};

export type ChatModelProvider = "deepseek" | "volcengine";

export interface ChatModelOption {
  provider: ChatModelProvider;
  model: string;
  label: string;
  description: string;
  key: string;
}

export function getDeepSeekModel(env: NodeJS.ProcessEnv = process.env): string {
  const configuredModel = env.DEEPSEEK_MODEL?.trim();

  return configuredModel || DEFAULT_DEEPSEEK_MODEL;
}

export function getVolcEngineModel(env: NodeJS.ProcessEnv = process.env): string {
  const configuredModel = env.VOLCENGINE_MODEL?.trim();

  return configuredModel || DEFAULT_VOLCENGINE_MODEL;
}

export function getModelOptions(env: NodeJS.ProcessEnv = process.env): ChatModelOption[] {
  const volcengineModels = getVolcEngineModels(env);

  return [
    {
      provider: "deepseek",
      model: getDeepSeekModel(env),
      label: `DeepSeek  ${getDeepSeekModel(env)}`,
      description: "使用 DeepSeek 兼容接口处理代码任务",
      key: "D",
    },
    ...volcengineModels.map((model, index) => ({
      provider: "volcengine",
      model,
      label: VOLCENGINE_MODEL_LABELS[model] ?? model,
      description: "使用火山方舟 OpenAI 兼容接口",
      key: index === 0 ? "V" : String(index + 1),
    } satisfies ChatModelOption)),
  ];
}

function getVolcEngineModels(env: NodeJS.ProcessEnv = process.env): string[] {
  const configuredModels = env.VOLCENGINE_MODELS?.split(",")
    .map((model) => model.trim())
    .filter((model) => model.length > 0);

  if (configuredModels?.length) {
    return configuredModels;
  }

  if (env.VOLCENGINE_MODEL?.trim()) {
    return [getVolcEngineModel(env)];
  }

  return DEFAULT_VOLCENGINE_MODELS;
}
