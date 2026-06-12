import {
  createVolcEngineEmbeddingProvider,
  type VolcEngineEmbeddingOptions,
} from "./embeddingProvider.js";

export interface VolcEngineBackendOptions extends VolcEngineEmbeddingOptions {
  endpoint?: string;
}

/**
 * 创建火山引擎记忆能力描述对象，兼容旧测试并暴露真实 embedding provider。
 * 新代码通过 embedding 字段调用火山方舟 embedding，描述信息用于配置检查。
 */
export function createVolcEngineBackend(options: VolcEngineBackendOptions) {
  const baseURL =
    options.baseURL ??
    options.endpoint ??
    process.env.VOLCENGINE_BASE_URL ??
    "https://ark.cn-beijing.volces.com/api/v3";
  return {
    provider: "volcengine" as const,
    model: options.model ?? process.env.VOLCENGINE_EMBEDDING_MODEL ?? "doubao-embedding",
    endpoint: baseURL,
    apiKey: options.apiKey ?? process.env.VOLCENGINE_API_KEY ?? "",
    embedding: createVolcEngineEmbeddingProvider({
      ...options,
      baseURL,
    }),
    describe() {
      return {
        provider: "volcengine" as const,
        model: this.model,
        endpoint: this.endpoint,
      };
    },
  };
}

