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

export const useAgentStore = defineStore("agent", () => {
  const isRunning = ref(false);
  const pendingApproval = ref<ApprovalRequest | null>(null);

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

  async function stopGeneration() {
    try {
      await invoke("stop_generation");
    } catch (e) {
      console.error("[agent] stop_generation error:", e);
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

  return {
    messages,
    isRunning,
    pendingApproval,
    configureAgent,
    sendMessage,
    stopGeneration,
    respondApproval,
  };
});
