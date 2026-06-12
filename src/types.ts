/**
 * Xuflow — 共享类型 + LLM Backend 接口
 *
 * 加新模型只需要:
 *   1. 新建 backends/xxx.ts 实现 LLMBackend 接口
 *   2. 在 loop.ts 里切换或路由
 */

// ---- Agent 模式 ----
export type AgentMode = "plan" | "act";

// ---- 工具定义（与协议无关的通用格式） ----
export interface ToolDef {
  name: string;
  description: string;
  parameters: {
    type: "object";
    properties: Record<string, ToolParam>;
    required?: string[];
  };
}

export interface ToolParam {
  type: "string" | "number" | "boolean";
  description: string;
}

// ---- 流事件（统一格式，屏蔽各家 SDK 差异） ----
export type StreamEvent =
  | { type: "text_delta"; text: string }
  | { type: "tool_use"; id: string; name: string; input: Record<string, unknown> }
  | {
      type: "done";
      usage: { inputTokens: number; outputTokens: number };
    };

// ---- 调用参数 ----
export interface ChatParams {
  systemPrompt: string;
  messages: LlmMessage[];
  tools: ToolDef[];
  maxTokens: number;
}

// ---- 消息格式（OpenAI 风格，最通用的表示） ----
export interface LlmMessage {
  role: "system" | "user" | "assistant" | "tool";
  content: string | null;
  tool_calls?: LlmToolCall[];
  tool_call_id?: string;
  name?: string;
}

export interface LlmToolCall {
  id: string;
  name: string;
  arguments: string; // JSON string
}

/**
 * 清洗消息序列：移除没有对应 tool 响应的孤立 tool_calls。
 * 防止数据库恢复的残缺会话导致 API 400 错误。
 */
export function sanitizeMessages(messages: LlmMessage[]): LlmMessage[] {
  const result: LlmMessage[] = [];
  const pendingToolIds = new Set<string>();

  for (const msg of messages) {
    if (msg.role === "assistant" && msg.tool_calls?.length) {
      // 记录此 assistant 消息需要的 tool_call_id
      for (const tc of msg.tool_calls) {
        pendingToolIds.add(tc.id);
      }
      result.push(msg);
    } else if (msg.role === "tool" && msg.tool_call_id) {
      pendingToolIds.delete(msg.tool_call_id);
      result.push(msg);
    } else {
      result.push(msg);
    }
  }

  // 如果结尾有未解决的 tool_calls，回退处理：删除这些 assistant 消息的 tool_calls
  if (pendingToolIds.size > 0) {
    for (let i = result.length - 1; i >= 0; i--) {
      const msg = result[i];
      if (msg.role === "assistant" && msg.tool_calls?.length) {
        const orphaned = msg.tool_calls.filter((tc) =>
          pendingToolIds.has(tc.id)
        );
        if (orphaned.length > 0) {
          // 移除孤立的 tool_calls
          const remaining = msg.tool_calls.filter(
            (tc) => !pendingToolIds.has(tc.id)
          );
          if (remaining.length === 0) {
            // 没有剩余 tool_calls：如果也没有文本内容，直接删除整条消息
            if (msg.content?.trim()) {
              result[i] = { ...msg, tool_calls: undefined };
            } else {
              result.splice(i, 1);
            }
          } else {
            result[i] = { ...msg, tool_calls: remaining };
          }
          for (const tc of orphaned) {
            pendingToolIds.delete(tc.id);
          }
        }
      }
    }
  }

  return result;
}

// ---- LLM Backend 接口 ----
export interface LLMBackend {
  /** 模型名 */
  model: string;
  /** 流式对话，返回统一事件流 */
  chat(params: ChatParams): AsyncGenerator<StreamEvent>;
}
