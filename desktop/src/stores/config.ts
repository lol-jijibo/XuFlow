import { defineStore } from "pinia";
import { ref, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { SelectOption, SelectGroupOption } from "naive-ui";

export type Provider = "deepseek" | "volcengine";

export interface TokenEstimateConfig {
  cjkCoeff: number;        // CJK 字符系数，默认 1.3
  nonCjkCoeff: number;     // 非 CJK 字符系数，默认 0.25
  structuredCoeff: number; // 结构化内容系数，默认 0.5
}

export interface ModelOption {
  label: string;
  value: string;
  provider: Provider;
  /** 默认的 API model ID / endpoint ID */
  apiModelId: string;
  /** Context window size in tokens (default: 128000 for all current models) */
  contextWindow?: number;
  /** Optional token estimation coefficient overrides for this model */
  tokenEstimateConfig?: TokenEstimateConfig;
}

export const ALL_MODELS: ModelOption[] = [
  // ── DeepSeek 官方 ──
  { label: "DeepSeek-V4-flash", value: "deepseek-v4-flash", provider: "deepseek", apiModelId: "deepseek-v4-flash" },
  { label: "DeepSeek-V4-pro",   value: "deepseek-v4-pro",   provider: "deepseek", apiModelId: "deepseek-v4-pro" },
  { label: "DeepSeek-V3.2",     value: "deepseek-v3.2",     provider: "deepseek", apiModelId: "deepseek-v3.2" },

  // ── 火山引擎 · DeepSeek 系列 （填入你的接入点 ID）──
  { label: "DeepSeek-V4-flash", value: "volc-deepseek-v4-flash", provider: "volcengine", apiModelId: "ep-xxxxxxxxxxxx" },
  { label: "DeepSeek-V4-pro",   value: "volc-deepseek-v4-pro",   provider: "volcengine", apiModelId: "ep-xxxxxxxxxxxx" },
  { label: "DeepSeek-V3.2",     value: "volc-deepseek-v3.2",     provider: "volcengine", apiModelId: "ep-xxxxxxxxxxxx" },

  // ── 火山引擎 · 豆包系列 （填入你的接入点 ID）──
  { label: "Doubao-Seed-2.0-Code",  value: "doubao-seed-2.0-code",  provider: "volcengine", apiModelId: "ep-xxxxxxxxxxxx" },
  { label: "Doubao-Seed-1.8",       value: "doubao-seed-1.8",       provider: "volcengine", apiModelId: "ep-xxxxxxxxxxxx" },
  { label: "Doubao-Seed-2.0-lite",  value: "doubao-seed-2.0-lite",  provider: "volcengine", apiModelId: "ep-xxxxxxxxxxxx" },
  { label: "Doubao-Seed-2.0-mini",  value: "doubao-seed-2.0-mini",  provider: "volcengine", apiModelId: "ep-xxxxxxxxxxxx" },
  { label: "Doubao-Seed-2.0-pro",   value: "doubao-seed-2.0-pro",   provider: "volcengine", apiModelId: "ep-xxxxxxxxxxxx" },
  { label: "Doubao-Seed-Character", value: "doubao-seed-character", provider: "volcengine", apiModelId: "ep-xxxxxxxxxxxx" },

  // ── 火山引擎 · GLM 系列 ──
  { label: "GLM-4.7", value: "glm-4.7", provider: "volcengine", apiModelId: "ep-xxxxxxxxxxxx" },
];

function getProviderForModel(modelId: string): Provider {
  const found = ALL_MODELS.find((m) => m.value === modelId);
  return found?.provider ?? "deepseek";
}

function getModelDisplayName(modelId: string): string {
  const found = ALL_MODELS.find((m) => m.value === modelId);
  return found?.label ?? modelId;
}

const CONFIG_STORAGE_KEY = "xuflow-config";

function loadConfig(): {
  activeModelId: string;
  deepseekApiKey: string;
  volcengineApiKey: string;
  modelEndpoints: Record<string, string>;
  contextWindows: Record<string, number>;
  minUserTurns: number;
  tokenEstimateConfigs: Record<string, TokenEstimateConfig>;
} {
  try {
    const raw = localStorage.getItem(CONFIG_STORAGE_KEY);
    if (raw) {
      const data = JSON.parse(raw);
      return {
        activeModelId: data.activeModelId ?? "volc-deepseek-v4-pro",
        deepseekApiKey: data.deepseekApiKey ?? "",
        volcengineApiKey: data.volcengineApiKey ?? "",
        modelEndpoints: data.modelEndpoints ?? {},
        contextWindows: data.contextWindows ?? {},
        minUserTurns: data.minUserTurns ?? 3,
        tokenEstimateConfigs: data.tokenEstimateConfigs ?? {},
      };
    }
  } catch (e) {
    console.error("[config] Failed to load state from localStorage:", e);
  }
  return {
    activeModelId: "volc-deepseek-v4-pro",
    deepseekApiKey: "",
    volcengineApiKey: "",
    modelEndpoints: {},
    contextWindows: {},
    minUserTurns: 3,
    tokenEstimateConfigs: {},
  };
}

function saveConfig(state: {
  activeModelId: string;
  deepseekApiKey: string;
  volcengineApiKey: string;
  modelEndpoints: Record<string, string>;
  contextWindows: Record<string, number>;
  minUserTurns: number;
  tokenEstimateConfigs: Record<string, TokenEstimateConfig>;
}) {
  try {
    localStorage.setItem(CONFIG_STORAGE_KEY, JSON.stringify(state));
  } catch (e) {
    console.error("[config] Failed to save state to localStorage:", e);
  }
}

export const useConfigStore = defineStore("config", () => {
  const saved = loadConfig();
  const activeModelId = ref(saved.activeModelId);
  const deepseekApiKey = ref(saved.deepseekApiKey);
  const volcengineApiKey = ref(saved.volcengineApiKey);

  /** 每个模型的接入点 ID 映射 (value → endpoint ID)，覆盖 ALL_MODELS 里的默认值 */
  const modelEndpoints = ref<Record<string, string>>(saved.modelEndpoints);

  /** Per-model context window overrides (modelValue → tokens) */
  const contextWindows = ref<Record<string, number>>(saved.contextWindows);
  /** Minimum user turns to preserve during context trimming */
  const minUserTurns = ref<number>(saved.minUserTurns);
  /** Per-model token estimation coefficient overrides (modelValue → config) */
  const tokenEstimateConfigs = ref<Record<string, TokenEstimateConfig>>(saved.tokenEstimateConfigs);

  function persist() {
    saveConfig({
      activeModelId: activeModelId.value,
      deepseekApiKey: deepseekApiKey.value,
      volcengineApiKey: volcengineApiKey.value,
      modelEndpoints: modelEndpoints.value,
      contextWindows: contextWindows.value,
      minUserTurns: minUserTurns.value,
      tokenEstimateConfigs: tokenEstimateConfigs.value,
    });
  }

  /** Initialize API keys from system environment variables (DEEP_SEEK_API_KEY, ARK_API_KEY).
   *  Only fills in keys that are currently empty — never overwrites user-saved values.
   *  Call once on app startup. Safe to call multiple times. */
  async function initFromEnv() {
    try {
      const env = await invoke<{ deepseek_api_key: string; ark_api_key: string }>("get_env_api_keys");
      // Only set if the ref is still empty (env acts as first-launch fallback)
      if (!deepseekApiKey.value && env.deepseek_api_key) {
        deepseekApiKey.value = env.deepseek_api_key;
        console.log("[config] Loaded DEEP_SEEK_API_KEY from environment");
      }
      if (!volcengineApiKey.value && env.ark_api_key) {
        volcengineApiKey.value = env.ark_api_key;
        console.log("[config] Loaded ARK_API_KEY from environment");
      }
      persist();
    } catch (e) {
      // No-op: env vars are optional; user can always enter keys in Settings
      console.log("[config] No env API keys found (this is fine)");
    }
  }

  /** Current provider derived from active model */
  const activeProvider = computed<Provider>(() =>
    getProviderForModel(activeModelId.value)
  );

  /** API key for the currently active provider */
  const activeApiKey = computed<string>(() =>
    activeProvider.value === "deepseek" ? deepseekApiKey.value : volcengineApiKey.value
  );

  /** 显示用的模型名称 */
  const activeModelName = computed<string>(() =>
    getModelDisplayName(activeModelId.value)
  );

  /** 发送给 API 的实际 model: 优先用 endpoint ID，否则用默认 apiModelId */
  const activeApiModelId = computed<string>(() => {
    const ep = modelEndpoints.value[activeModelId.value];
    if (ep) return ep;
    const found = ALL_MODELS.find((m) => m.value === activeModelId.value);
    return found?.apiModelId ?? activeModelId.value;
  });

  /** Current active model's context window size. Custom override first, then model default, then 128K. */
  const activeContextWindow = computed<number>(() => {
    const custom = contextWindows.value[activeModelId.value];
    if (custom != null && custom > 0) return custom;
    const found = ALL_MODELS.find((m) => m.value === activeModelId.value);
    return found?.contextWindow ?? 128000;
  });

  /** Current active model's minimum user turns for trimming. */
  const activeMinUserTurns = computed<number>(() => minUserTurns.value);

  /** Current active model's token estimation config. */
  const activeTokenEstimateConfig = computed<TokenEstimateConfig>(() => {
    const custom = tokenEstimateConfigs.value[activeModelId.value];
    if (custom) return custom;
    const found = ALL_MODELS.find((m) => m.value === activeModelId.value);
    return found?.tokenEstimateConfig ?? { cjkCoeff: 1.3, nonCjkCoeff: 0.25, structuredCoeff: 0.5 };
  });

  /** Set a custom context window for a model. */
  function setContextWindow(modelId: string, window: number) {
    contextWindows.value = { ...contextWindows.value, [modelId]: window };
    persist();
  }

  /** Get the effective context window for a model. */
  function getContextWindow(modelId: string): number {
    const custom = contextWindows.value[modelId];
    if (custom != null && custom > 0) return custom;
    const found = ALL_MODELS.find((m) => m.value === modelId);
    return found?.contextWindow ?? 128000;
  }

  /** Set the minimum user turns for all models. */
  function setMinUserTurns(n: number) {
    minUserTurns.value = Math.max(1, Math.min(10, n));
    persist();
  }

  /** Set custom token estimation config for a model. */
  function setTokenEstimateConfig(modelId: string, config: TokenEstimateConfig) {
    tokenEstimateConfigs.value = { ...tokenEstimateConfigs.value, [modelId]: config };
    persist();
  }

  /** 所有火山引擎模型的 endpoint 列表（用于设置页面） */
  const volcModels = computed(() =>
    ALL_MODELS.filter((m) => m.provider === "volcengine")
  );

  /** Grouped model options for NSelect */
  const modelOptions = computed<Array<SelectOption | SelectGroupOption>>(() => [
    {
      type: "group",
      label: "DeepSeek 官方",
      key: "deepseek",
      children: ALL_MODELS
        .filter((m) => m.provider === "deepseek")
        .map((m) => ({ label: m.label, value: m.value })),
    },
    {
      type: "group",
      label: "火山引擎 · DeepSeek 系列",
      key: "volc-deepseek",
      children: ALL_MODELS
        .filter((m) => m.provider === "volcengine" && m.label.startsWith("DeepSeek"))
        .map((m) => ({ label: m.label, value: m.value })),
    },
    {
      type: "group",
      label: "火山引擎 · 豆包系列",
      key: "volc-doubao",
      children: ALL_MODELS
        .filter((m) => m.provider === "volcengine" && m.label.startsWith("Doubao"))
        .map((m) => ({ label: m.label, value: m.value })),
    },
    {
      type: "group",
      label: "火山引擎 · GLM 系列",
      key: "volc-glm",
      children: ALL_MODELS
        .filter((m) => m.provider === "volcengine" && m.label.startsWith("GLM"))
        .map((m) => ({ label: m.label, value: m.value })),
    },
  ]);

  function setActiveModelId(id: string) {
    activeModelId.value = id;
    persist();
  }

  function setDeepseekApiKey(key: string) {
    deepseekApiKey.value = key;
    persist();
  }

  function setVolcengineApiKey(key: string) {
    volcengineApiKey.value = key;
    persist();
  }

  function setModelEndpoint(modelValue: string, endpointId: string) {
    modelEndpoints.value = { ...modelEndpoints.value, [modelValue]: endpointId };
    persist();
  }

  function getModelEndpoint(modelValue: string): string {
    return modelEndpoints.value[modelValue] ?? "";
  }

  // Auto-persist whenever any key config value changes (handles v-model, direct ref mutation, etc.)
  watch([activeModelId, deepseekApiKey, volcengineApiKey, modelEndpoints, contextWindows, minUserTurns, tokenEstimateConfigs], () => {
    persist();
  }, { deep: true });

  return {
    activeModelId,
    deepseekApiKey,
    volcengineApiKey,
    modelEndpoints,
    activeProvider,
    activeApiKey,
    activeModelName,
    activeApiModelId,
    modelOptions,
    volcModels,
    setActiveModelId,
    setDeepseekApiKey,
    setVolcengineApiKey,
    setModelEndpoint,
    getModelEndpoint,
    getProviderForModel,
    getModelDisplayName,
    initFromEnv,
    // Context management
    contextWindows,
    minUserTurns,
    tokenEstimateConfigs,
    activeContextWindow,
    activeMinUserTurns,
    activeTokenEstimateConfig,
    setContextWindow,
    getContextWindow,
    setMinUserTurns,
    setTokenEstimateConfig,
  };
});
