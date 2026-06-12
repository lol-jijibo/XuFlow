import OpenAI from "openai";

export interface EmbeddingProvider {
  size: number;
  embed(input: EmbeddingInput): Promise<number[]>;
}

export interface VolcEngineEmbeddingOptions {
  apiKey?: string;
  model?: string;
  baseURL?: string;
  size?: number;
  mode?: "text" | "multimodal";
}

export type MultimodalEmbeddingInput =
  | { type: "text"; text: string }
  | { type: "image_url"; image_url: { url: string } };

export type EmbeddingInput = string | MultimodalEmbeddingInput[];

/**
 * 创建火山引擎 embedding 提供者，负责把文本转换成 Qdrant 可写入向量。
 * 内部复用 OpenAI 兼容 SDK，模型与网关地址通过环境变量解耦。
 */
export function createVolcEngineEmbeddingProvider(
  options: VolcEngineEmbeddingOptions = {}
): EmbeddingProvider {
  const model =
    options.model ?? process.env.VOLCENGINE_EMBEDDING_MODEL ?? "doubao-embedding";
  const apiKey = options.apiKey ?? process.env.VOLCENGINE_API_KEY ?? "";
  const baseURL =
    options.baseURL ??
    process.env.VOLCENGINE_BASE_URL ??
    "https://ark.cn-beijing.volces.com/api/v3";
  const mode =
    options.mode ??
    (process.env.VOLCENGINE_EMBEDDING_MODE === "multimodal"
      ? "multimodal"
      : "text");
  const client = new OpenAI({
    apiKey,
    baseURL,
  });

  return {
    size: options.size ?? Number(process.env.VOLCENGINE_EMBEDDING_SIZE ?? 2048),
    async embed(input: EmbeddingInput) {
      if (mode === "multimodal") {
        return embedMultimodal(baseURL, apiKey, model, input);
      }
      const response = await client.embeddings.create({
        model,
        input: normalizeTextInput(input),
      });
      return response.data[0]?.embedding ?? [];
    },
  };
}

async function embedMultimodal(
  baseURL: string,
  apiKey: string,
  model: string,
  input: EmbeddingInput
): Promise<number[]> {
  const response = await fetch(
    `${baseURL.replace(/\/$/, "")}/embeddings/multimodal`,
    {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${apiKey}`,
      },
      body: JSON.stringify({
        model,
        input: normalizeMultimodalInput(input),
      }),
    }
  );
  if (!response.ok) {
    throw new Error(
      `VolcEngine multimodal embedding failed: ${response.status} ${await response.text()}`
    );
  }
  const body = (await response.json()) as {
    data?: { embedding?: number[] } | { embedding?: number[] }[];
  };
  if (Array.isArray(body.data)) {
    return body.data[0]?.embedding ?? [];
  }
  return body.data?.embedding ?? [];
}

function normalizeTextInput(input: EmbeddingInput): string {
  if (typeof input === "string") return input;
  return input
    .filter((item) => item.type === "text")
    .map((item) => item.text)
    .join("\n");
}

function normalizeMultimodalInput(
  input: EmbeddingInput
): MultimodalEmbeddingInput[] {
  if (typeof input === "string") {
    return input.trim() ? [{ type: "text", text: input }] : [];
  }
  return input.filter((item) => item.type !== "text" || !!item.text?.trim());
}

/**
 * 创建本地确定性 embedding，用于没有火山密钥时保持 Qdrant 链路可运行。
 * 该实现按字符哈希生成固定维度向量，便于本地开发和测试稳定复现。
 */
export function createLocalEmbeddingProvider(size = 64): EmbeddingProvider {
  return {
    size,
    async embed(text: string) {
      const vector = new Array<number>(size).fill(0);
      for (let i = 0; i < text.length; i++) {
        const index = text.charCodeAt(i) % size;
        vector[index] += 1;
      }
      const length = Math.sqrt(
        vector.reduce((sum, value) => sum + value * value, 0)
      );
      return length === 0 ? vector : vector.map((value) => value / length);
    },
  };
}

/**
 * 根据环境变量选择 embedding 来源，优先使用火山引擎真实 embedding。
 * 未配置火山密钥时降级到本地向量，避免开发环境无法启动。
 */
export function createEmbeddingProvider(env: NodeJS.ProcessEnv = process.env) {
  if (env.VOLCENGINE_API_KEY && env.VOLCENGINE_EMBEDDING_MODEL) {
    return createVolcEngineEmbeddingProvider({
      apiKey: env.VOLCENGINE_API_KEY,
      model: env.VOLCENGINE_EMBEDDING_MODEL,
      baseURL: env.VOLCENGINE_BASE_URL,
      mode: env.VOLCENGINE_EMBEDDING_MODE === "multimodal" ? "multimodal" : "text",
      size: env.VOLCENGINE_EMBEDDING_SIZE
        ? Number(env.VOLCENGINE_EMBEDDING_SIZE)
        : undefined,
    });
  }
  return createLocalEmbeddingProvider();
}
