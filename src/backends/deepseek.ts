/**
 * Xuflow — DeepSeek Backend
 *
 * 通过 OpenAI 兼容 SDK 连接 DeepSeek API。
 * 模型: deepseek-chat (V3) — 性价比最高的工具调用模型
 *
 * 以后加 Anthropic backend: 新建 backends/anthropic.ts，实现同样的 LLMBackend 接口
 * 以后加 Ollama backend:   新建 backends/ollama.ts，一样实现接口
 */

import OpenAI from "openai";
import type { LLMBackend, ChatParams, StreamEvent, LlmMessage } from "../types.js";
import { sanitizeMessages } from "../types.js";

export class DeepSeekBackend implements LLMBackend {
  readonly model: string;
  private client: OpenAI;

  constructor(opts?: { apiKey?: string; model?: string }) {
    this.model = opts?.model ?? "deepseek-chat";

    this.client = new OpenAI({
      apiKey: opts?.apiKey ?? process.env.DEEPSEEK_API_KEY!,
      baseURL: "https://api.deepseek.com",
    });
  }

  async *chat(params: ChatParams): AsyncGenerator<StreamEvent> {
    const stream = await this.client.chat.completions.create({
      model: this.model,
      max_tokens: params.maxTokens,
      // DeepSeek 的 system prompt 放在 messages 里
      messages: [
        { role: "system", content: params.systemPrompt },
        ...sanitizeMessages(params.messages).map(toOpenAI),
      ],
      tools: params.tools.map(toOpenAITool),
      stream: true,
    });

    let inputTokens = 0;
    let outputTokens = 0;

    // ---- 流式累积：文本 + 工具调用 ----
    let textBuffer = "";
    const toolCalls: Map<
      number,
      { id: string; name: string; args: string }
    > = new Map();

    for await (const chunk of stream) {
      // Token 计数（在第一个 chunk 和最后一个 chunk）
      if (chunk.usage) {
        inputTokens = chunk.usage.prompt_tokens ?? 0;
        outputTokens = chunk.usage.completion_tokens ?? 0;
      }

      const delta = chunk.choices[0]?.delta;
      if (!delta) continue;

      // 文本增量
      if (delta.content) {
        textBuffer += delta.content;
        yield { type: "text_delta", text: delta.content };
      }

      // 工具调用增量
      if (delta.tool_calls) {
        for (const tc of delta.tool_calls) {
          const idx = tc.index;
          if (!toolCalls.has(idx)) {
            toolCalls.set(idx, { id: tc.id ?? "", name: "", args: "" });
          }
          const entry = toolCalls.get(idx)!;
          if (tc.id) entry.id = tc.id;
          if (tc.function?.name) entry.name += tc.function.name;
          if (tc.function?.arguments) entry.args += tc.function.arguments;
        }
      }
    }

    // ---- 工具调用已完成，逐个 yield ----
    for (const [, tc] of toolCalls) {
      let input: Record<string, unknown> = {};
      try {
        input = JSON.parse(tc.args);
      } catch {
        // 参数解析失败，传空对象让工具自己报错
      }
      yield {
        type: "tool_use",
        id: tc.id,
        name: tc.name,
        input,
      };
    }

    // ---- 最终事件 ----
    yield {
      type: "done",
      usage: { inputTokens, outputTokens },
    };
  }
}

// ---- 内部格式转换 ----

/** 把通用消息转为 OpenAI 格式 */
function toOpenAI(
  msg: LlmMessage
): OpenAI.Chat.Completions.ChatCompletionMessageParam {
  switch (msg.role) {
    case "assistant":
      return {
        role: "assistant",
        content: msg.content,
        ...(msg.tool_calls && {
          tool_calls: msg.tool_calls.map((tc) => ({
            id: tc.id,
            type: "function" as const,
            function: {
              name: tc.name,
              arguments: tc.arguments,
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

/** 把通用工具定义转为 OpenAI function 格式 */
function toOpenAITool(
  t: ChatParams["tools"][number]
): OpenAI.Chat.Completions.ChatCompletionTool {
  return {
    type: "function",
    function: {
      name: t.name,
      description: t.description,
      parameters: t.parameters as Record<string, unknown>,
    },
  };
}
