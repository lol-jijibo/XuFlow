import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useAgentStore, ToolCall, ToolResult, type TodoItem, type PlanProposal } from "../stores/agent";
import { useProjectStore, type ToolCallEntry } from "../stores/project";
import { trySummarizeConversation } from "./useConversationSummary";

export function useTauriEvent() {
  const agentStore = useAgentStore();
  const projectStore = useProjectStore();

  let persistTimer: ReturnType<typeof setTimeout> | null = null;

  /** Accumulated unlisten functions — cleared on teardown. */
  let unlisteners: UnlistenFn[] = [];
  /** Guard to ensure listeners are only registered once per composable instance. */
  let listenersSetup = false;

  /** MySQL 模式下，向 MySQL 插入新消息行并记录 _dbId。 */
  async function dbEnsureMessage(msg: ReturnType<typeof lastStreamingMsg>) {
    if (!msg || !projectStore.dbConnected) return;
    if (msg._dbId) return;
    const tcJson = msg.toolCalls?.length
      ? JSON.stringify(msg.toolCalls.map(t => ({ id: t.id, name: t.name, arguments: t.arguments, result: t.result, resultDone: t.resultDone })))
      : undefined;
    const id = await projectStore.dbAddMessage(
      projectStore.activeConversationId ?? "",
      msg.role,
      msg.content,
      msg.reasoning,
      tcJson,
    );
    if (id) msg._dbId = id;
  }

  /** 向 MySQL 更新已存在消息行的最新内容。 */
  async function dbSyncMessage(msg: ReturnType<typeof lastStreamingMsg>) {
    if (!msg || !msg._dbId || !projectStore.dbConnected) return;
    const fields: Record<string, unknown> = {
      content: msg.content,
      done: msg.done,
      reasoning: msg.reasoning ?? null,
      reasoning_done: msg.reasoningDone ?? false,
    };
    if (msg.toolCalls?.length) {
      fields.tool_calls = msg.toolCalls.map(t => ({
        id: t.id, name: t.name, arguments: t.arguments, result: t.result, resultDone: t.resultDone,
      }));
    }
    await projectStore.dbUpdateMessage(msg._dbId, fields);
  }

  /** Throttled persist — MySQL 模式下实时插入/更新，localStorage 模式下节流写入。 */
  function schedulePersist() {
    const msg = lastStreamingMsg();
    if (msg && projectStore.dbConnected) {
      dbEnsureMessage(msg);
      if (persistTimer) return;
      persistTimer = setTimeout(async () => {
        const current = lastStreamingMsg();
        if (current) await dbSyncMessage(current);
        persistTimer = null;
      }, 1500);
    } else {
      if (persistTimer) return;
      persistTimer = setTimeout(() => {
        projectStore.persistMessages();
        persistTimer = null;
      }, 1000);
    }
  }

  function flushPersist() {
    if (persistTimer) {
      clearTimeout(persistTimer);
      persistTimer = null;
    }
    const msg = lastStreamingMsg();
    if (msg && projectStore.dbConnected) {
      dbSyncMessage(msg);
    } else {
      projectStore.persistMessages();
    }
  }

  /** Helper: get the last assistant message (must be streaming, i.e. done=false). */
  function lastStreamingMsg() {
    const conv = projectStore.activeConversation;
    if (!conv) return null;
    const msgs = conv.messages;
    const last = msgs[msgs.length - 1];
    if (last && last.role === "assistant" && !last.done) return last;
    return null;
  }

  /** Helper: ensure toolCalls array exists on the given message. */
  function ensureToolCalls(msg: ReturnType<typeof lastStreamingMsg>): ToolCallEntry[] {
    if (!msg) return [];
    if (!msg.toolCalls) msg.toolCalls = [];
    return msg.toolCalls;
  }

  async function setupListeners() {
    // Idempotent: if already set up, tear down old listeners first to avoid duplicates.
    if (listenersSetup) {
      teardownListeners();
    }
    listenersSetup = true;

    // ── Streaming text deltas ──
    unlisteners.push(
      await listen<string>("agent:text-delta", (event) => {
        const msg = lastStreamingMsg();
        if (!msg) return;
        msg.content += event.payload;
        schedulePersist();
      })
    );

    // ── Reasoning / thinking deltas ──
    unlisteners.push(
      await listen<string>("agent:reasoning-delta", (event) => {
        const msg = lastStreamingMsg();
        if (!msg) return;
        if (msg.reasoning === undefined) msg.reasoning = "";
        msg.reasoning += event.payload;
        schedulePersist();
      })
    );

    // ── Reasoning complete ──
    unlisteners.push(
      await listen<string>("agent:reasoning-done", (_event) => {
        const msg = lastStreamingMsg();
        if (!msg) return;
        msg.reasoningDone = true;
        schedulePersist();
      })
    );

    // ── Tool call started ──
    unlisteners.push(
      await listen<string>("agent:tool-call", (event) => {
        const tc: ToolCall = JSON.parse(event.payload);
        const msg = lastStreamingMsg();
        if (!msg) return;
        const toolCalls = ensureToolCalls(msg);

        // Parse arguments for structured display
        let argsParsed: Record<string, unknown> | undefined;
        try {
          argsParsed = JSON.parse(tc.arguments);
        } catch { /* keep raw string */ }

        toolCalls.push({
          id: tc.id,
          name: tc.name,
          arguments: tc.arguments,
          argsParsed,
          resultDone: false,
        });
        schedulePersist();

        // Track file path for file-related tools
        if (tc.name === "read_file" || tc.name === "write_file") {
          if (argsParsed?.path) {
            agentStore.setLastFilePath(String(argsParsed.path));
          }
        }
      })
    );

    // ── Tool result received ──
    unlisteners.push(
      await listen<string>("agent:tool-result", (event) => {
        const tr: ToolResult = JSON.parse(event.payload);
        const msg = lastStreamingMsg();
        if (!msg || !msg.toolCalls) return;

        const entry = msg.toolCalls.find((t) => t.id === tr.id);
        if (entry) {
          entry.result = tr.content;
          entry.resultDone = true;
        }
        schedulePersist();
      })
    );

    // ── Approval required ──
    unlisteners.push(
      await listen<string>("agent:approval-required", (event) => {
        const approval = JSON.parse(event.payload);
        agentStore.pendingApproval = approval;
      })
    );

    // ── Todo list update ──
    unlisteners.push(
      await listen<string>("agent:todo-update", (event) => {
        try {
          const todos: TodoItem[] = JSON.parse(event.payload);
          agentStore.todos = todos;
        } catch { /* ignore parse errors */ }
      })
    );

    // ── Plan proposal ──
    unlisteners.push(
      await listen<string>("agent:plan-proposed", (event) => {
        try {
          const plan: PlanProposal = JSON.parse(event.payload);
          agentStore.pendingPlan = plan;
        } catch { /* ignore parse errors */ }
      })
    );

    // ── Token usage update (before/after each API call) ──
    unlisteners.push(
      await listen<string>("agent:token-usage", (event) => {
        try {
          const d = JSON.parse(event.payload);
          if (d.phase === "before") {
            agentStore.tokenUsage = d.estimated;
          } else if (d.phase === "after") {
            if (d.actual != null) {
              agentStore.tokenActual = d.actual;
              // Calibrate: use the larger of estimated vs actual
              agentStore.tokenUsage = Math.max(d.estimated, d.actual);
            }
          }
          agentStore.contextWindow = d.context_window ?? agentStore.contextWindow;
          agentStore.contextRemaining = d.context_remaining ?? (agentStore.contextWindow - agentStore.tokenUsage);
        } catch { /* ignore parse errors */ }
      })
    );

    // ── Context trimmed notification ──
    unlisteners.push(
      await listen<string>("agent:context-trimmed", (event) => {
        try {
          const d = JSON.parse(event.payload);
          agentStore.contextTrimmed = true;
          agentStore.trimMeta = {
            roundsRemoved: d.rounds_removed ?? 0,
            tokensFreed: d.tokens_freed ?? 0,
          };
          agentStore.contextWindow = d.context_window ?? agentStore.contextWindow;
          // Auto-clear the transient badge after 3 seconds
          setTimeout(() => {
            agentStore.contextTrimmed = false;
          }, 3000);
        } catch { /* ignore parse errors */ }
      })
    );

    // ── Agent loop done ──
    unlisteners.push(
      await listen<string>("agent:done", (event) => {
        // Version-aware usage parsing (v=1: { v: 1, usage: { total_tokens, ... } })
        try {
          const p = JSON.parse(event.payload);
          if (p.v === 1 && p.usage?.total_tokens != null) {
            agentStore.tokenUsage = p.usage.total_tokens;
            agentStore.tokenActual = p.usage.total_tokens;
          }
        } catch { /* old empty payload — safe to ignore */ }

        const conv = projectStore.activeConversation;
        const project = projectStore.activeProject;
        if (conv) {
          const msgs = conv.messages;
          const lastMsg = msgs[msgs.length - 1];
          if (lastMsg && lastMsg.role === "assistant") {
            lastMsg.done = true;
          }
          // Reveal hidden conversation in sidebar
          if (conv.visible === false && project) {
            projectStore.revealConversation(project.id, conv.id);
          }
        }
        agentStore.isRunning = false;
        flushPersist();

        // Trigger conversation summary (non-blocking)
        trySummarizeConversation().catch((e) =>
          console.warn("[summary] trySummarizeConversation error:", e)
        );
      })
    );

    // ── Agent error ──
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

  /** Clean up all registered listeners. */
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
