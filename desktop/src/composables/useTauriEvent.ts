import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useAgentStore, ToolCall, ToolResult } from "../stores/agent";
import { useProjectStore } from "../stores/project";

export function useTauriEvent() {
  const agentStore = useAgentStore();
  const projectStore = useProjectStore();

  let persistTimer: ReturnType<typeof setTimeout> | null = null;

  /** Accumulated unlisten functions — cleared on teardown. */
  let unlisteners: UnlistenFn[] = [];
  /** Guard to ensure listeners are only registered once per composable instance. */
  let listenersSetup = false;

  /** Throttled persist — saves at most once per second during streaming */
  function schedulePersist() {
    if (persistTimer) return;
    persistTimer = setTimeout(() => {
      projectStore.persistMessages();
      persistTimer = null;
    }, 1000);
  }

  function flushPersist() {
    if (persistTimer) {
      clearTimeout(persistTimer);
      persistTimer = null;
    }
    projectStore.persistMessages();
  }

  async function setupListeners() {
    // Idempotent: if already set up, tear down old listeners first to avoid duplicates.
    if (listenersSetup) {
      teardownListeners();
    }
    listenersSetup = true;

    // Streaming text deltas from the Rust backend
    unlisteners.push(
      await listen<string>("agent:text-delta", (event) => {
        const conv = projectStore.activeConversation;
        if (!conv) return;
        const msgs = conv.messages;
        const lastMsg = msgs[msgs.length - 1];
        if (lastMsg && lastMsg.role === "assistant" && !lastMsg.done) {
          lastMsg.content += event.payload;
          schedulePersist();
        }
      })
    );

    // Tool call started
    unlisteners.push(
      await listen<string>("agent:tool-call", (event) => {
        const tc: ToolCall = JSON.parse(event.payload);
        const conv = projectStore.activeConversation;
        if (!conv) return;
        const msgs = conv.messages;
        const lastMsg = msgs[msgs.length - 1];
        if (lastMsg && lastMsg.role === "assistant") {
          lastMsg.content += `\n\n> 🔧 **${tc.name}** \`${tc.arguments.slice(0, 120)}\`\n`;
          schedulePersist();
        }
      })
    );

    // Tool result received
    unlisteners.push(
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
          schedulePersist();
        }
      })
    );

    // Approval required
    unlisteners.push(
      await listen<string>("agent:approval-required", (event) => {
        const approval = JSON.parse(event.payload);
        agentStore.pendingApproval = approval;
      })
    );

    // Agent loop done
    unlisteners.push(
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
        flushPersist();
      })
    );

    // Agent error
    unlisteners.push(
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
        flushPersist();
      })
    );
  }

  /** Clean up all registered listeners. Call on component unmount. */
  function teardownListeners() {
    for (const fn of unlisteners) {
      fn();
    }
    unlisteners = [];
    listenersSetup = false;
    if (persistTimer) {
      clearTimeout(persistTimer);
      persistTimer = null;
    }
  }

  return { setupListeners, teardownListeners };
}
