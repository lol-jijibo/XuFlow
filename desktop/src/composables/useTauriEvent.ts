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
      msg.done,
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

  /** 立即刷新当前消息到持久层，确保最终状态不丢失。
   *  流式结束后 lastStreamingMsg 因 done=true 返回 null，
   *  但仍需将最终的 done/reasoningDone 等字段写入数据库。 */
  function flushPersist() {
    if (persistTimer) {
      clearTimeout(persistTimer);
      persistTimer = null;
    }
    const msg = lastStreamingMsg();
    if (msg && projectStore.dbConnected) {
      dbSyncMessage(msg);
    } else if (projectStore.dbConnected) {
      // 消息已标记完成 → lastStreamingMsg 返回 null，但最终状态仍未持久化
      const conv = projectStore.activeConversation;
      const last = conv?.messages[conv.messages.length - 1];
      if (last && last.role === "assistant" && last.done && last._dbId) {
        dbSyncMessage(last);
      }
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
    // 新一轮思考开始时，之前可能已收到过 reasoning-done，需要重置标记。
    // 否则 ReasoningBlock 会在思考仍在流式输出时显示"思考已完成"。
    unlisteners.push(
      await listen<string>("agent:reasoning-delta", (event) => {
        const msg = lastStreamingMsg();
        if (!msg) return;
        if (msg.reasoning === undefined) msg.reasoning = "";
        // 新一轮思考开始：清除上一轮的完成标记，让 UI 正确显示"思考中..."
        if (msg.reasoningDone) msg.reasoningDone = false;
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
      await listen<string>("agent:done", async (event) => {
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
            // 兜底：如果存在思考内容但 reasoningDone 未标记，确保完成标记被设置。
            // 防止 reasoning-done 事件异常丢失导致重载后显示"思考中"。
            if (lastMsg.reasoning && !lastMsg.reasoningDone) {
              lastMsg.reasoningDone = true;
            }
            // AI 彻底生成完毕后自动收起思考块，避免占据过多空间
            // 仅当用户未手动操作时自动收起（undefined = 未操作过）
            if (lastMsg.reasoningExpanded === undefined) {
              lastMsg.reasoningExpanded = false;
            }
          }
        }
        agentStore.isRunning = false;
        flushPersist();

        // 先提炼标题再显示会话，确保侧边栏展示时已是 AI 生成的标题
        if (conv && project && conv.visible === false) {
          try {
            await trySummarizeConversation();
          } catch (e) {
            console.warn("[summary] Failed to generate title:", e);
          }
          // 标题提炼完成后再显示（即使失败也显示，用默认标题兜底）
          projectStore.revealConversation(project.id, conv.id);
        } else {
          // 已可见的会话：非阻塞触发标题更新
          trySummarizeConversation().catch((e) =>
            console.warn("[summary] trySummarizeConversation error:", e)
          );
        }
      })
    );

    // ── Agent error ──
    unlisteners.push(
      await listen<string>("agent:error", (event) => {
        const conv = projectStore.activeConversation;
        const project = projectStore.activeProject;
        if (conv) {
          const msgs = conv.messages;
          const lastMsg = msgs[msgs.length - 1];
          if (lastMsg && lastMsg.role === "assistant") {
            lastMsg.content += `\n[Error: ${event.payload}]`;
            lastMsg.done = true;
            // 出错时同样自动收起思考块
            if (lastMsg.reasoningExpanded === undefined) {
              lastMsg.reasoningExpanded = false;
            }
          }
          // 出错时也显示隐藏的会话，避免用户发送了消息但会话永远不可见
          if (conv.visible === false && project) {
            projectStore.revealConversation(project.id, conv.id);
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
