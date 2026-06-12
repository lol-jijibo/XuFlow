import OpenAI from "openai";
import type { ChatParams, LLMBackend, LlmMessage, StreamEvent } from "../types.js";
import { sanitizeMessages } from "../types.js";

export interface VolcEngineChatBackendOptions {
  apiKey?: string;
  model?: string;
  baseURL?: string;
}

/**
 * 火山引擎聊天后端，负责通过火山方舟 OpenAI 兼容接口生成回答。
 * 它实现统一 LLMBackend 契约，供入口层按 provider 环境变量切换。
 */
export class VolcEngineChatBackend implements LLMBackend {
  readonly model: string;
  private client: OpenAI;

  constructor(options: VolcEngineChatBackendOptions = {}) {
    this.model = options.model ?? process.env.VOLCENGINE_MODEL ?? "doubao-pro";
    this.client = new OpenAI({
      apiKey: options.apiKey ?? process.env.VOLCENGINE_API_KEY ?? "",
      baseURL:
        options.baseURL ??
        process.env.VOLCENGINE_BASE_URL ??
        "https://ark.cn-beijing.volces.com/api/v3",
    });
  }

  async *chat(params: ChatParams): AsyncGenerator<StreamEvent> {
    const stream = await this.client.chat.completions.create({
      model: this.model,
      max_tokens: params.maxTokens,
      messages: [
        { role: "system", content: params.systemPrompt },
        ...sanitizeMessages(params.messages).map(toOpenAI),
      ],
      tools: params.tools.map(toOpenAITool),
      stream: true,
    });

    let inputTokens = 0;
    let outputTokens = 0;
    const toolCalls: Map<number, { id: string; name: string; args: string }> =
      new Map();

    for await (const chunk of stream) {
      if (chunk.usage) {
        inputTokens = chunk.usage.prompt_tokens ?? 0;
        outputTokens = chunk.usage.completion_tokens ?? 0;
      }
      const delta = chunk.choices[0]?.delta;
      if (!delta) continue;
      if (delta.content) {
        yield { type: "text_delta", text: delta.content };
      }
      if (delta.tool_calls) {
        for (const toolCall of delta.tool_calls) {
          const index = toolCall.index;
          if (!toolCalls.has(index)) {
            toolCalls.set(index, { id: toolCall.id ?? "", name: "", args: "" });
          }
          const entry = toolCalls.get(index)!;
          if (toolCall.id) entry.id = toolCall.id;
          if (toolCall.function?.name) entry.name += toolCall.function.name;
          if (toolCall.function?.arguments) entry.args += toolCall.function.arguments;
        }
      }
    }

    for (const [, toolCall] of toolCalls) {
      let input: Record<string, unknown> = {};
      try {
        input = JSON.parse(toolCall.args);
      } catch {
        input = {};
      }
      yield {
        type: "tool_use",
        id: toolCall.id,
        name: toolCall.name,
        input,
      };
    }

    yield {
      type: "done",
      usage: { inputTokens, outputTokens },
    };
  }
}

function toOpenAI(
  msg: LlmMessage
): OpenAI.Chat.Completions.ChatCompletionMessageParam {
  switch (msg.role) {
    case "assistant":
      return {
        role: "assistant",
        content: msg.content,
        ...(msg.tool_calls && {
          tool_calls: msg.tool_calls.map((toolCall) => ({
            id: toolCall.id,
            type: "function" as const,
            function: {
              name: toolCall.name,
              arguments: toolCall.arguments,
            },
          })),
        }),
      };
    case "tool":
      return {
        role: "tool",
        tool_call_id: msg.tool_call_id!,
        content: msg.content ?? "",
      };
    case "user":
    case "system":
    default:
      return {
        role: msg.role as "user" | "system",
        content: msg.content ?? "",
      };
  }
}

function toOpenAITool(
  tool: ChatParams["tools"][number]
): OpenAI.Chat.Completions.ChatCompletionTool {
  return {
    type: "function",
    function: {
      name: tool.name,
      description: tool.description,
      parameters: tool.parameters as Record<string, unknown>,
    },
  };
}

