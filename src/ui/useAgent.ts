/**
 * Xuflow 鈥?useAgent Hook
 *
 * 灏佽浜嗘暣涓?agent 瀵硅瘽寰幆鐨勭姸鎬佺鐞?
 *   - 娴佸紡鏂囨湰绱Н
 *   - 宸ュ叿璋冪敤鎵ц
 *   - 瀹℃壒娴佺▼锛圥romise 鎸傝捣 鈫?鐢ㄦ埛纭 鈫?缁х画锛?
 *   - LlmMessage[] 鍜?UIMessage[] 鍙岃建鍚屾
 */

import { useState, useRef, useCallback, useMemo } from "react";
import { sanitizeMessages, type LLMBackend, type LlmMessage, type AgentMode } from "../types.js";
import { getToolsForMode, executeTool, DANGEROUS_TOOLS } from "../tools.js";
import type { ConversationMemory } from "../memory/index.js";

export interface ToolCallEntry {
  id: string;
  name: string;
  args: Record<string, unknown>;
  result?: string;
  status: "running" | "done" | "denied" | "error";
}

export interface UIMessage {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  toolCalls?: ToolCallEntry[];
  usage?: { inputTokens: number; outputTokens: number };
}

export type AgentStatus =
  | "idle"
  | "streaming"
  | "executing"
  | "awaiting_approval";

export interface ApprovalRequest {
  tool: string;
  description: string;
  command?: string;
}

let msgId = 0;

function syncMsgCounter(existing: UIMessage[]) {
  for (const msg of existing) {
    const match = msg.id.match(/^msg_(\d+)$/);
    if (match) {
      msgId = Math.max(msgId, parseInt(match[1], 10));
    }
  }
}

function nextId() {
  return `msg_${++msgId}`;
}

export function useAgent(
  backend: LLMBackend,
  baseSystemPrompt: string,
  mode: AgentMode,
  memory?: ConversationMemory
) {
  const [messages, setMessages] = useState<UIMessage[]>(() => {
    const existing = memory?.uiMessages ?? [];
    syncMsgCounter(existing);
    return existing;
  });
  const [status, setStatus] = useState<AgentStatus>("idle");
  const [streaming, setStreaming] = useState("");
  const [pendingApproval, setPendingApproval] =
    useState<ApprovalRequest | null>(null);
  const [totalTokens, setTotalTokens] = useState({
    input: 0,
    output: 0,
  });

  const activeTools = useMemo(() => getToolsForMode(mode), [mode]);
  const systemPrompt = useMemo(() => {
    const modeBlock =
      mode === "plan"
        ? `\n\n【当前模式 PLAN 只读规划】你现在处于 Plan 模式，只能阅读和分析代码，不能修改文件或执行命令。可用工具: read_file, list_dir, grep。
收到用户请求后，先梳理现状，再给出分步实施计划，最后提示切换到 Act 模式。`
        : `\n\n【当前模式 ACT 执行】你现在处于 Act 模式，可以使用全部工具，包括写文件和运行命令。遵循计划逐步执行，修改前确认，改完后验证。`;
    return baseSystemPrompt + modeBlock;
  }, [baseSystemPrompt, mode]);

  const approvalRef = useRef<((approved: boolean) => void) | null>(null);
  const llmMessagesRef = useRef<LlmMessage[]>(
    memory?.llmMessages ? sanitizeMessages([...memory.llmMessages]) : []
  );

  const requestApproval = useCallback((req: ApprovalRequest): Promise<boolean> => {
    return new Promise((resolve) => {
      setPendingApproval(req);
      setStatus("awaiting_approval");
      approvalRef.current = resolve;
    });
  }, []);

  const resolveApproval = useCallback((approved: boolean) => {
    setPendingApproval(null);
    approvalRef.current?.(approved);
    approvalRef.current = null;
  }, []);

  const addMessage = useCallback((content: string) => {
    const msg: UIMessage = {
      id: nextId(),
      role: "system",
      content,
    };
    setMessages((prev) => [...prev, msg]);
  }, []);

  const sendMessage = useCallback(
    async (userInput: string) => {
      if (status !== "idle") return;

      const userMsg: UIMessage = {
        id: nextId(),
        role: "user",
        content: userInput,
      };
      setMessages((prev) => [...prev, userMsg]);
      llmMessagesRef.current.push({ role: "user", content: userInput });
      await memory?.appendUserMessage(userMsg, { role: "user", content: userInput });
      setStatus("streaming");

      let loop = 0;
      const MAX_LOOPS = 15;

      try {
        while (loop < MAX_LOOPS) {
          loop++;

          const stream = backend.chat({
            systemPrompt,
            messages: llmMessagesRef.current,
            tools: activeTools,
            maxTokens: 4096,
          });

          let textContent = "";
          const toolUses: {
            id: string;
            name: string;
            input: Record<string, unknown>;
          }[] = [];
          let usage = { inputTokens: 0, outputTokens: 0 };

          setStreaming("");
          setStatus("streaming");

          for await (const event of stream) {
            switch (event.type) {
              case "text_delta":
                textContent += event.text;
                setStreaming((prev) => prev + event.text);
                break;
              case "tool_use":
                toolUses.push({
                  id: event.id,
                  name: event.name,
                  input: event.input,
                });
                break;
              case "done":
                usage = event.usage;
                break;
            }
          }

          setTotalTokens((prev) => ({
            input: prev.input + usage.inputTokens,
            output: prev.output + usage.outputTokens,
          }));

          if (toolUses.length === 0) {
            const assistantMsg: UIMessage = {
              id: nextId(),
              role: "assistant",
              content: textContent,
              usage,
            };
            setMessages((prev) => [...prev, assistantMsg]);
            const llmMsg = {
              role: "assistant" as const,
              content: textContent,
            };
            llmMessagesRef.current.push(llmMsg);
            await memory?.appendAssistantMessage(assistantMsg, llmMsg);
            setStreaming("");
            break;
          }

          const toolCallEntries: ToolCallEntry[] = toolUses.map((tc) => ({
            id: tc.id,
            name: tc.name,
            args: tc.input,
            status: "running" as const,
          }));

          const assistantMsg: UIMessage = {
            id: nextId(),
            role: "assistant",
            content: textContent,
            toolCalls: toolCallEntries,
            usage,
          };
          setMessages((prev) => [...prev, assistantMsg]);
          setStreaming("");

          const assistantLlmMsg: LlmMessage = {
            role: "assistant",
            content: textContent || null,
            tool_calls: toolUses.map((tc) => ({
              id: tc.id,
              name: tc.name,
              arguments: JSON.stringify(tc.input),
            })),
          };
          llmMessagesRef.current.push(assistantLlmMsg);
          await memory?.appendAssistantMessage(assistantMsg, assistantLlmMsg);

          for (const tc of toolUses) {
            if (DANGEROUS_TOOLS.has(tc.name)) {
              const desc =
                tc.name === "bash"
                  ? String((tc.input as { description?: string }).description ?? "无描述")
                  : tc.name === "write_file"
                    ? `写入: ${String((tc.input as { path?: string }).path ?? "?")}`
                    : tc.name;
              const command =
                tc.name === "bash"
                  ? String((tc.input as { command?: string }).command ?? "")
                  : undefined;

              const approved = await requestApproval({
                tool: tc.name,
                description: desc,
                command,
              });

              if (!approved) {
                setMessages((prev) => {
                  const last = prev[prev.length - 1];
                  if (last.toolCalls) {
                    const entry = last.toolCalls.find((e) => e.id === tc.id);
                    if (entry) {
                      entry.status = "denied";
                      entry.result = "用户取消了此操作";
                    }
                  }
                  return [...prev];
                });
                const deniedMsg: LlmMessage = {
                  role: "tool",
                  tool_call_id: tc.id,
                  name: tc.name,
                  content: "[denied] 用户取消了此操作",
                };
                llmMessagesRef.current.push(deniedMsg);
                await memory?.appendToolMessage(deniedMsg);
                await memory?.updateToolResult(
                  tc.id,
                  "[denied] 用户取消了此操作",
                  "denied"
                );
                continue;
              }
            }

            let result: string;
            try {
              result = await executeTool(tc.name, tc.input);
            } catch (err: any) {
              result = `[error] 宸ュ叿鎵ц寮傚父: ${err.message}`;
            }

            const isError = result.startsWith("[error]");
            const isDenied = result.startsWith("[denied]");

            setMessages((prev) => {
              const last = prev[prev.length - 1];
              if (last.toolCalls) {
                const tce = last.toolCalls.find((t) => t.id === tc.id);
                if (tce) {
                  tce.result = result;
                  tce.status = isDenied
                    ? "denied"
                    : isError
                      ? "error"
                      : "done";
                }
              }
              return [...prev];
            });

            const toolMsg: LlmMessage = {
              role: "tool",
              tool_call_id: tc.id,
              name: tc.name,
              content: result,
            };
            llmMessagesRef.current.push(toolMsg);
            await memory?.appendToolMessage(toolMsg);
            await memory?.updateToolResult(
              tc.id,
              result,
              isDenied ? "denied" : isError ? "error" : "done"
            );
          }
        }
      } catch (err: any) {
        const errorMsg: UIMessage = {
          id: nextId(),
          role: "assistant",
          content: `错误: ${err.message}`,
        };
        setMessages((prev) => [...prev, errorMsg]);
        setStreaming("");
      } finally {
        setStatus("idle");
        setStreaming("");
      }
    },
    [backend, systemPrompt, activeTools, status, requestApproval, memory]
  );

  const resetSession = useCallback(() => {
    msgId = 0;
    setMessages([]);
    setStreaming("");
    setTotalTokens({ input: 0, output: 0 });
    llmMessagesRef.current = [];
  }, []);

  return {
    messages,
    status,
    streaming,
    pendingApproval,
    totalTokens,
    sendMessage,
    resolveApproval,
    addMessage,
    resetSession,
  } as const;
}
