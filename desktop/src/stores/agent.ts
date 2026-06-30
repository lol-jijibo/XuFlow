import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useProjectStore } from "./project";
import { useConfigStore } from "./config";

export interface ApprovalRequest {
  tool: string;
  params: string;
}

export interface ToolCall {
  id: string;
  name: string;
  arguments: string;
}

export interface ToolResult {
  id: string;
  content: string;
}

export interface TodoItem {
  content: string;
  status: "pending" | "in_progress" | "completed";
}

export interface PlanProposal {
  title: string;
  steps: string[];
  files_to_modify: string[];
}

export const useAgentStore = defineStore("agent", () => {
  const isRunning = ref(false);
  const pendingApproval = ref<ApprovalRequest | null>(null);
  /** File path most recently touched by a read_file or write_file tool call. */
  const lastFilePath = ref<string | null>(null);
  /** Current todo list from the agent's todo_write tool. */
  const todos = ref<TodoItem[]>([]);
  /** Pending plan proposal from the agent's propose_plan tool. */
  const pendingPlan = ref<PlanProposal | null>(null);
  /** Whether Plan Mode is enabled. */
  const planMode = ref(false);

  // ── Token / context tracking ────────────────────────────────────────
  /** Current estimated token usage (heuristic, updated by token-usage events). */
  const tokenUsage = ref<number>(0);
  /** Last API-reported actual token usage (from agent:done). */
  const tokenActual = ref<number | null>(null);
  /** Current model's context window size (updated by backend events). */
  const contextWindow = ref<number>(128000);
  /** Estimated remaining context capacity. */
  const contextRemaining = ref<number>(128000);

  /** Whether the context was recently trimmed (transient, auto-clears). */
  const contextTrimmed = ref<boolean>(false);
  /** Metadata about the last trim operation. */
  const trimMeta = ref<{ roundsRemoved: number; tokensFreed: number } | null>(null);

  /** Percentage of context used (0–100). */
  const tokenUsagePercent = computed(() => {
    if (contextWindow.value <= 0) return 0;
    return Math.min(100, Math.round((tokenUsage.value / contextWindow.value) * 100));
  });

  /** Warning level for context usage visualization. */
  const tokenWarningLevel = computed<'green' | 'yellow' | 'orange' | 'red'>(() => {
    const pct = tokenUsagePercent.value;
    if (pct < 50) return 'green';
    if (pct < 80) return 'yellow';
    if (pct < 90) return 'orange';
    return 'red';
  });

  /** Current active messages — delegates to project store's activeConversation */
  const messages = computed({
    get: () => useProjectStore().activeMessages,
    set: () => {
      // no-op: writing goes through the project store's conversation directly
    },
  });

  /**
   * Push the current config (api key, provider, model) to the Rust backend.
   * Must be called before the first send_message and whenever config changes.
   */
  async function configureAgent() {
    const config = useConfigStore();
    // Pull API keys from env vars on first launch (no-op if already set)
    await config.initFromEnv();
    try {
      await invoke("configure_agent", {
        apiKey: config.activeApiKey,
        provider: config.activeProvider,
        model: config.activeApiModelId,
      });
      console.log("[agent] configure_agent sent:", {
        provider: config.activeProvider,
        model: config.activeModelName,
      });
    } catch (e) {
      console.error("[agent] configure_agent error:", e);
    }
  }

  async function sendMessage(content: string) {
    const projectStore = useProjectStore();
    const conv = projectStore.activeConversation;
    if (!conv) {
      console.error("[agent] No active conversation");
      return;
    }

    for (const msg of conv.messages) {
      if (msg.role === "assistant" && !msg.done) {
        msg.done = true;
        if (msg.reasoning && !msg.reasoningDone) {
          msg.reasoningDone = true;
        }
        if (msg.reasoningExpanded === undefined) {
          msg.reasoningExpanded = false;
        }
      }
    }

    conv.messages.push({ role: "user", content, done: true });
    conv.messages.push({ role: "assistant", content: "", done: false });
    conv.updatedAt = Date.now();
    projectStore.persistMessages();
    isRunning.value = true;

    try {
      const result = await invoke<string>("send_message", { content });
      console.log("[agent] send_message done:", result);
    } catch (e) {
      console.error("[agent] send_message error:", e);
      const msgs = conv.messages;
      const lastMsg = msgs[msgs.length - 1];
      if (lastMsg && lastMsg.role === "assistant") {
        lastMsg.content += `\n[Error: ${e}]`;
        lastMsg.done = true;
      }
    } finally {
      projectStore.persistMessages();
      isRunning.value = false;
    }
  }

  // 停止生成：通知 Rust 后端取消后，立即更新前端 UI 状态，
  // 避免等待后端完全结束才切换按钮（造成"点了没反应"的体验）。
  async function stopGeneration() {
    try {
      await invoke("stop_generation");
    } catch (e) {
      console.error("[agent] stop_generation error:", e);
    }
    // 立即将 UI 切换回可发送状态，同时标记最后一条助手消息为已完成，
    // 防止依赖 agent:done 事件（被取消时事件转发器已停止，Done 事件可能丢失）。
    isRunning.value = false;
    const conv = useProjectStore().activeConversation;
    if (conv) {
      const lastMsg = conv.messages[conv.messages.length - 1];
      if (lastMsg && lastMsg.role === "assistant" && !lastMsg.done) {
        lastMsg.done = true;
      }
    }
  }

  async function respondApproval(approved: boolean) {
    try {
      await invoke("respond_approval", { approved });
    } catch (e) {
      console.error("[agent] respond_approval error:", e);
    } finally {
      pendingApproval.value = null;
    }
  }

  /** Update the last-known file path (called from tool-call listener). */
  function setLastFilePath(path: string | null) {
    lastFilePath.value = path;
  }

  return {
    messages,
    isRunning,
    pendingApproval,
    lastFilePath,
    todos,
    pendingPlan,
    planMode,
    setLastFilePath,
    configureAgent,
    sendMessage,
    stopGeneration,
    respondApproval,
    // Token / context tracking
    tokenUsage,
    tokenActual,
    contextWindow,
    contextRemaining,
    contextTrimmed,
    trimMeta,
    tokenUsagePercent,
    tokenWarningLevel,
  };
});
