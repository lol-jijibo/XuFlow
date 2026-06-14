import { listen } from "@tauri-apps/api/event";
import { useAgentStore, ToolCall, ToolResult } from "../stores/agent";
import { useProjectStore } from "../stores/project";

export function useTauriEvent() {
  const agentStore = useAgentStore();
  const projectStore = useProjectStore();

  async function setupListeners() {
    // Streaming text deltas from the Rust backend
    await listen<string>("agent:text-delta", (event) => {
      const conv = projectStore.activeConversation;
      if (!conv) return;
      const msgs = conv.messages;
      const lastMsg = msgs[msgs.length - 1];
      if (lastMsg && lastMsg.role === "assistant" && !lastMsg.done) {
        lastMsg.content += event.payload;
      }
    });

    // Tool call started
    await listen<string>("agent:tool-call", (event) => {
      const tc: ToolCall = JSON.parse(event.payload);
      const conv = projectStore.activeConversation;
      if (!conv) return;
      const msgs = conv.messages;
      const lastMsg = msgs[msgs.length - 1];
      if (lastMsg && lastMsg.role === "assistant") {
        lastMsg.content += `\n\n> 🔧 **${tc.name}** \`${tc.arguments.slice(0, 120)}\`\n`;
      }
    });

    // Tool result received
    await listen<string>("agent:tool-result", (event) => {
      const tr: ToolResult = JSON.parse(event.payload);
      const conv = projectStore.activeConversation;
      if (!conv) return;
      const msgs = conv.messages;
      const lastMsg = msgs[msgs.length - 1];
      if (lastMsg && lastMsg.role === "assistant") {
        const preview =
          tr.content.length > 300
            ? tr.content.slice(0, 300) + "\n... (truncated)"
            : tr.content;
        lastMsg.content += `\n<details><summary>Tool result</summary>\n\n\`\`\`\n${preview}\n\`\`\`\n</details>\n`;
      }
    });

    // Approval required
    await listen<string>("agent:approval-required", (event) => {
      const approval = JSON.parse(event.payload);
      agentStore.pendingApproval = approval;
    });

    // Agent loop done
    await listen("agent:done", () => {
      const conv = projectStore.activeConversation;
      if (conv) {
        const msgs = conv.messages;
        const lastMsg = msgs[msgs.length - 1];
        if (lastMsg && lastMsg.role === "assistant") {
          lastMsg.done = true;
        }
      }
      agentStore.isRunning = false;
    });

    // Agent error
    await listen<string>("agent:error", (event) => {
      const conv = projectStore.activeConversation;
      if (conv) {
        const msgs = conv.messages;
        const lastMsg = msgs[msgs.length - 1];
        if (lastMsg && lastMsg.role === "assistant") {
          lastMsg.content += `\n[Error: ${event.payload}]`;
          lastMsg.done = true;
        }
      }
      agentStore.isRunning = false;
    });
  }

  return { setupListeners };
}
