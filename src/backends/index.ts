import type { LLMBackend } from "../types.js";
import { DeepSeekBackend } from "./deepseek.js";
import { VolcEngineChatBackend } from "./volcengine.js";
import {
  getDeepSeekModel,
  getVolcEngineModel,
  type ChatModelOption,
} from "../modelConfig.js";

/**
 * 根据环境变量创建聊天后端，负责在 DeepSeek 与火山引擎之间切换。
 * 默认继续使用 DeepSeek，设置 LLM_PROVIDER=volcengine 后走火山方舟。
 */
export function createBackend(
  env: NodeJS.ProcessEnv = process.env,
  selection?: Pick<ChatModelOption, "provider" | "model">
): LLMBackend {
  const provider = selection?.provider ?? env.LLM_PROVIDER?.trim().toLowerCase();

  if (provider === "volcengine") {
    return new VolcEngineChatBackend({
      apiKey: env.VOLCENGINE_API_KEY,
      model: selection?.model ?? getVolcEngineModel(env),
      baseURL: env.VOLCENGINE_BASE_URL,
    });
  }
  return new DeepSeekBackend({
    apiKey: env.DEEPSEEK_API_KEY,
    model: selection?.model ?? getDeepSeekModel(env),
  });
}
