import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface ToolCallEntry {
  id: string;
  name: string;
  arguments: string;
  /** Parsed arguments object (cached after first parse). */
  argsParsed?: Record<string, unknown>;
  result?: string;
  /** Whether the tool result has been received. */
  resultDone: boolean;
}

export interface ChatMessage {
  role: "user" | "assistant" | "system";
  content: string;
  done: boolean;
  /** Reasoning / thinking content streamed by the model (e.g. DeepSeek-R1 reasoning). */
  reasoning?: string;
  /** Whether the reasoning block is complete. */
  reasoningDone?: boolean;
  /** UI state: whether the user has expanded the reasoning block. */
  reasoningExpanded?: boolean;
  /** Tool calls made during this assistant turn. Not serialized — rebuilt from events. */
  toolCalls?: ToolCallEntry[];
  /** SQLite 行 ID（自增主键，用于流式更新定位）。 */
  _dbId?: number;
}

export interface Conversation {
  id: string;
  title: string;
  /** How the title was set: 'default' (新会话 N), 'auto' (AI summary), 'manual' (user typed) */
  titleSource?: "default" | "auto" | "manual";
  /** Whether the conversation is visible in the sidebar. Hidden until first AI response completes. */
  visible?: boolean;
  messages: ChatMessage[];
  createdAt: number;
  updatedAt: number;
  /** 软删除时间戳：非空表示在回收站中。 */
  deletedAt?: number | null;
  /** 归属项目 ID：回收站中需要此字段以显示来源项目名。 */
  projectId?: string;
}

export interface Project {
  id: string;
  name: string;
  path?: string;
  source: "local" | "imported";
  /** 是否置顶，置顶项目始终排在列表最前面。 */
  pinned?: boolean;
  conversations: Conversation[];
  createdAt: number;
  updatedAt: number;
}

let nextId = 1;
function uid(): string {
  return `${Date.now()}-${nextId++}`;
}

const STORAGE_KEY = "xuflow-projects";
const PINNED_KEY = "xuflow-pinned-projects";

function finishStaleStreamingMessages(messages: ChatMessage[]) {
  for (const msg of messages) {
    if (msg.role !== "assistant" || msg.done) continue;
    msg.done = true;
    if (msg.reasoning && !msg.reasoningDone) {
      msg.reasoningDone = true;
    }
    if (msg.reasoningExpanded === undefined) {
      msg.reasoningExpanded = false;
    }
  }
}

// ── 置顶项目 ID 持久化（独立于主存储，SQLite 模式下也生效）──

function loadPinnedIds(): Set<string> {
  try {
    const raw = localStorage.getItem(PINNED_KEY);
    if (raw) {
      const arr = JSON.parse(raw);
      if (Array.isArray(arr)) return new Set(arr);
    }
  } catch (e) {
    console.error("[project] Failed to load pinned ids:", e);
  }
  return new Set();
}

function savePinnedIds(ids: Set<string>) {
  try {
    localStorage.setItem(PINNED_KEY, JSON.stringify([...ids]));
  } catch (e) {
    console.error("[project] Failed to save pinned ids:", e);
  }
}

// ── localStorage 工具函数（SQLite 异常时的回退方案）────────

function loadFromLocalStorage(): { projects: Project[]; activeProjectId: string | null; activeConversationId: string | null } {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const data = JSON.parse(raw);
      return {
        projects: data.projects ?? [],
        activeProjectId: data.activeProjectId ?? null,
        activeConversationId: data.activeConversationId ?? null,
      };
    }
  } catch (e) {
    console.error("[project] Failed to load state from localStorage:", e);
  }
  return { projects: [], activeProjectId: null, activeConversationId: null };
}

function saveToLocalStorage(
  projects: Project[],
  activeProjectId: string | null,
  activeConversationId: string | null
) {
  try {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ projects, activeProjectId, activeConversationId })
    );
  } catch (e) {
    console.error("[project] Failed to save state to localStorage:", e);
  }
}

// ── SQLite 数据加载 ───────────────────────────────────────────

/** 从 SQLite 加载所有项目及其会话和消息，重组为前端 Project[] 结构。 */
async function loadFromMySql(): Promise<{ projects: Project[]; activeProjectId: string | null; activeConversationId: string | null }> {
  try {
    // 检查是否已迁移（暂未使用，后续可做迁移提示）
    await invoke<boolean>("db_is_migrated").catch(() => false);

    // 从 MySQL 加载项目列表
    const dbProjects = await invoke<any[]>("db_list_projects");

    const projects: Project[] = [];
    for (const p of dbProjects) {
      // 加载该项目下的会话
      const dbSessions = await invoke<any[]>("db_list_sessions", { projectId: p.id });

      const conversations: Conversation[] = [];
      for (const s of dbSessions) {
        // 加载会话消息
        const dbMessages = await invoke<any[]>("db_get_messages", { sessionId: s.id });

        const messages: ChatMessage[] = dbMessages.map((m: any) => ({
          role: m.role,
          content: m.content,
          done: m.done,
          reasoning: m.reasoning ?? undefined,
          reasoningDone: m.reasoning_done,
          toolCalls: m.tool_calls ? JSON.parse(m.tool_calls) : undefined,
        }));
        finishStaleStreamingMessages(messages);

        conversations.push({
          id: s.id,
          title: s.title,
          titleSource: (s.title_source as any) ?? "default",
          visible: s.visible,
          deletedAt: s.deleted_at ?? null,
          projectId: p.id,
          messages,
          createdAt: s.created_at,
          updatedAt: s.updated_at,
        });
      }

      projects.push({
        id: p.id,
        name: p.name,
        path: p.path ?? undefined,
        source: (p.source as any) ?? "local",
        conversations,
        createdAt: p.created_at,
        updatedAt: p.updated_at,
      });
    }

    // 每次启动桌面端都展示空白新会话界面，不恢复活跃会话 ID。
    // 活跃项目取第一个项目，若无项目则让 on-boarding 逻辑创建默认项目。
    // 修复：原 `localState.activeProjectId` 引用了一个不存在的变量，
    // 导致 loadFromMySql() 每次都抛 ReferenceError，应用始终回退到过期的 localStorage 数据。
    const activeProjectId = projects[0]?.id ?? null;
    const activeConversationId = null;

    return { projects, activeProjectId, activeConversationId };
  } catch (e) {
    console.error("[project] Failed to load from MySQL:", e);
    throw e;
  }
}

// ── Store ───────────────────────────────────────────────────

export const useProjectStore = defineStore("project", () => {
  const saved = loadFromLocalStorage();
  const projects = ref<Project[]>(saved.projects);
  const activeProjectId = ref<string | null>(saved.activeProjectId);
  // 每次启动桌面端时开启空白新会话，不恢复上次的活跃会话 ID，
  // 让用户在中间的聊天框直接输入即可，ChatPanel 会在首次发送消息时自动创建新会话
  const activeConversationId = ref<string | null>(null);

  /** SQLite 是否已就绪（启动即连接，始终为 true）。 */
  const dbConnected = ref(true);

  /** 回收站中的会话列表，从 SQLite 加载。 */
  const trashConversations = ref<Conversation[]>([]);

  /** 回收站是否在侧边栏中展开。 */
  const trashExpanded = ref(false);

  /** 从 SQLite 加载回收站数据。 */
  async function loadTrashFromDb() {
    try {
      const trashRows = await invoke<any[]>("db_list_trash_sessions");
      trashConversations.value = trashRows.map((s: any) => ({
        id: s.id,
        title: s.title,
        titleSource: (s.title_source as any) ?? "default",
        visible: s.visible,
        deletedAt: s.deleted_at,
        projectId: s.project_id,
        messages: [],
        createdAt: s.created_at,
        updatedAt: s.updated_at,
      }));
    } catch (e) {
      console.error("[project] Failed to load trash from SQLite:", e);
    }
  }

  /** 尝试从 SQLite 加载数据。成功则替换 projects 并返回 true。 */
  async function tryLoadFromMySql(): Promise<boolean> {
    // SQLite 始终可用，直接加载
    try {
      const data = await loadFromMySql();
      projects.value = data.projects;
      activeProjectId.value = data.activeProjectId;
      activeConversationId.value = data.activeConversationId;
      // 同步加载回收站数据
      await loadTrashFromDb();
      console.log("[project] Loaded from SQLite:", projects.value.length, "projects,", trashConversations.value.length, "in trash");
      return true;
    } catch (e) {
      console.error("[project] Failed to load from SQLite, falling back to localStorage:", e);
      return false;
    }
  }

  /** Validate that saved IDs still point to real objects; fall back to first available.
   *  Skips invisible conversations when picking a fallback.
   *  当 activeConversationId 为 null 时保留空状态（启动时展示空白新会话界面），
   *  仅在已指向某个会话但该会话不存在时才回退到首个可见会话。 */
  function validateState() {
    const project = projects.value.find((p) => p.id === activeProjectId.value);
    if (!project) {
      activeProjectId.value = projects.value[0]?.id ?? null;
    }
    // null 表示有意不选中任何会话（启动时展示空白对话），不需要回退
    if (activeConversationId.value === null) return;
    const activeProj = projects.value.find((p) => p.id === activeProjectId.value);
    const conv = activeProj?.conversations.find((c) => c.id === activeConversationId.value);
    if (!conv) {
      const visibleConvs = activeProj?.conversations.filter((c) => c.visible !== false) ?? [];
      activeConversationId.value = visibleConvs[0]?.id ?? activeProj?.conversations[0]?.id ?? null;
    }
  }

  validateState();

  /** 持久化：SQLite 已由各方法实时写入，同时更新 localStorage 作为安全回退。
   *  避免 SQLite 加载失败时回退到包含已删除会话的过期数据。 */
  function persist() {
    saveToLocalStorage(projects.value, activeProjectId.value, activeConversationId.value);
  }

  const activeProject = computed(() =>
    projects.value.find((p) => p.id === activeProjectId.value) ?? null
  );

  const activeConversation = computed(() =>
    activeProject.value?.conversations.find(
      (c) => c.id === activeConversationId.value
    ) ?? null
  );

  const activeMessages = computed(() =>
    activeConversation.value?.messages ?? []
  );

  /** 按置顶优先排序后的项目列表：置顶项目排在最前。 */
  const sortedProjects = computed(() => {
    const arr = [...projects.value];
    arr.sort((a, b) => {
      if (a.pinned && !b.pinned) return -1;
      if (!a.pinned && b.pinned) return 1;
      return 0;
    });
    return arr;
  });

  // ── 置顶状态初始化（从 localStorage 恢复到 projects 的 pinned 字段）──

  const pinnedIds = loadPinnedIds();
  for (const p of projects.value) {
    p.pinned = pinnedIds.has(p.id);
  }

  // ── 项目操作 ────────────────────────────────────────────

  function createProject(name: string): Project {
    const project: Project = {
      id: uid(),
      name,
      source: "local",
      conversations: [],
      createdAt: Date.now(),
      updatedAt: Date.now(),
    };
    projects.value.push(project);

    if (dbConnected.value) {
      // 将前端生成的 ID 传给后端，确保前后端使用同一个 ID
      invoke("db_create_project", { id: project.id, name, source: "local" })
        .then((row: any) => {
          if (row.id !== project.id) {
            console.warn("[project] Backend returned different project id, syncing:", project.id, "->", row.id);
            project.id = row.id;
          }
          project.createdAt = row.created_at;
          project.updatedAt = row.updated_at;
        })
        .catch((e) => console.error("[project] db_create_project failed:", e));
    }
    persist();
    return project;
  }

  function importProject(name: string, path: string): Project {
    const project: Project = {
      id: uid(),
      name,
      path,
      source: "imported",
      conversations: [],
      createdAt: Date.now(),
      updatedAt: Date.now(),
    };
    projects.value.push(project);

    if (dbConnected.value) {
      invoke("db_create_project", { id: project.id, name, source: "imported" })
        .then((row: any) => {
          if (row.id !== project.id) {
            console.warn("[project] Backend returned different project id, syncing:", project.id, "->", row.id);
            project.id = row.id;
          }
          project.createdAt = row.created_at;
          project.updatedAt = row.updated_at;
        })
        .catch((e) => console.error("[project] db_create_project failed:", e));
    }
    persist();
    return project;
  }

  function deleteProject(id: string) {
    const idx = projects.value.findIndex((p) => p.id === id);
    if (idx === -1) return;
    const removed = projects.value[idx];
    projects.value.splice(idx, 1);
    if (activeProjectId.value === id) {
      activeProjectId.value = projects.value[0]?.id ?? null;
      activeConversationId.value =
        projects.value[0]?.conversations[0]?.id ?? null;
    }

    if (dbConnected.value) {
      invoke<boolean>("db_delete_project", { id })
        .then((deleted) => {
          if (!deleted) {
            console.warn("[project] db_delete_project returned false — project not found in SQLite, restoring UI state");
            projects.value.splice(idx, 0, removed);
            activeProjectId.value = removed.id;
          }
        })
        .catch((e) => {
          console.error("[project] db_delete_project failed:", e);
          projects.value.splice(idx, 0, removed);
          activeProjectId.value = removed.id;
        });
    }
    persist();
  }

  // 修改项目名称，同步更新 MySQL 和 localStorage。
  function updateProjectName(projectId: string, name: string): boolean {
    const project = projects.value.find((p) => p.id === projectId);
    if (!project) return false;
    project.name = name;
    project.updatedAt = Date.now();

    if (dbConnected.value) {
      invoke("db_update_project_name", { id: projectId, name })
        .catch((e) => console.error("[project] db_update_project_name failed:", e));
    }
    persist();
    return true;
  }

  /** 切换项目置顶状态。置顶项目始终排在列表最前面。 */
  function pinProject(projectId: string): boolean {
    const project = projects.value.find((p) => p.id === projectId);
    if (!project) return false;
    project.pinned = !project.pinned;

    // 同步更新 localStorage 中的置顶 ID 集合
    const pinnedIds = loadPinnedIds();
    if (project.pinned) {
      pinnedIds.add(projectId);
    } else {
      pinnedIds.delete(projectId);
    }
    savePinnedIds(pinnedIds);

    // 不需要 SQLite 持久化（UI 偏好），仅触发响应式更新
    projects.value = [...projects.value];
    return true;
  }

  // ── 会话操作 ────────────────────────────────────────────

  function createConversation(projectId: string, title?: string, titleSource?: "default" | "manual", visible = true): Conversation {
    const project = projects.value.find((p) => p.id === projectId);
    if (!project) throw new Error(`Project ${projectId} not found`);
    // 隐藏会话（等 AI 回复后自动提炼标题再显示）不预设 "新会话 N" 标题
    const defaultTitle = visible ? `新会话 ${project.conversations.length + 1}` : "";
    const conv: Conversation = {
      id: uid(),
      title: title || defaultTitle,
      titleSource: titleSource ?? (title ? "manual" : "default"),
      visible,
      messages: [],
      createdAt: Date.now(),
      updatedAt: Date.now(),
    };
    project.conversations.push(conv);
    project.updatedAt = Date.now();

    if (dbConnected.value) {
      // 将前端生成的 ID 传给后端，确保前后端使用同一个 ID，
      // 避免后续删除/重命名操作因 ID 不匹配而静默失败
      invoke("db_create_session", {
        id: conv.id,
        projectId,
        title: conv.title,
        titleSource: conv.titleSource,
        visible,
      })
        .then((row: any) => {
          // 后端现在使用前端传入的 ID，但以防后端降级生成新 ID，仍做同步覆盖
          if (row.id !== conv.id) {
            console.warn("[project] Backend returned different session id, syncing:", conv.id, "->", row.id);
            conv.id = row.id;
          }
          conv.createdAt = row.created_at;
          conv.updatedAt = row.updated_at;
        })
        .catch((e) => console.error("[project] db_create_session failed:", e));
    }
    persist();
    return conv;
  }

  function deleteConversation(projectId: string, convId: string) {
    const project = projects.value.find((p) => p.id === projectId);
    if (!project) return;
    const idx = project.conversations.findIndex((c) => c.id === convId);
    if (idx === -1) return;
    // 软删除：先保存引用，从活跃列表移除，然后移入回收站
    const removed = project.conversations[idx];
    project.conversations.splice(idx, 1);
    project.updatedAt = Date.now();
    if (activeConversationId.value === convId) {
      activeConversationId.value = project.conversations[0]?.id ?? null;
    }

    if (dbConnected.value) {
      invoke<boolean>("db_delete_session", { id: convId })
        .then((deleted) => {
          if (deleted) {
            // 同步到本地回收站列表
            trashConversations.value.unshift({
              ...removed,
              deletedAt: Date.now(),
              projectId,
            });
          } else {
            // 软删除失败（ID 不匹配），回退前端状态
            console.warn("[project] db_delete_session returned false — restoring UI state");
            project.conversations.splice(idx, 0, removed);
            project.updatedAt = Date.now();
          }
        })
        .catch((e) => {
          console.error("[project] db_delete_session failed:", e);
          project.conversations.splice(idx, 0, removed);
          project.updatedAt = Date.now();
        });
    } else {
      // 无数据库时直接加入本地回收站
      trashConversations.value.unshift({
        ...removed,
        deletedAt: Date.now(),
        projectId,
      });
    }
    persist();
  }

  /** 从回收站恢复会话到原项目。 */
  function restoreConversation(convId: string, projectId: string): boolean {
    const trashIdx = trashConversations.value.findIndex((c) => c.id === convId);
    if (trashIdx === -1) return false;

    const project = projects.value.find((p) => p.id === projectId);
    if (!project) return false;

    const [restored] = trashConversations.value.splice(trashIdx, 1);
    restored.deletedAt = null;
    restored.projectId = undefined;
    project.conversations.push(restored);
    project.updatedAt = Date.now();

    if (dbConnected.value) {
      invoke<boolean>("db_restore_session", { id: convId })
        .then((ok) => {
          if (!ok) console.warn("[project] db_restore_session returned false");
        })
        .catch((e) => console.error("[project] db_restore_session failed:", e));
    }
    persist();
    return true;
  }

  /** 彻底删除会话（物理删除，不可恢复）。 */
  function permanentDeleteConversation(convId: string): boolean {
    const trashIdx = trashConversations.value.findIndex((c) => c.id === convId);
    if (trashIdx === -1) return false;

    trashConversations.value.splice(trashIdx, 1);

    if (dbConnected.value) {
      invoke<boolean>("db_permanent_delete_session", { id: convId })
        .then((ok) => {
          if (!ok) console.warn("[project] db_permanent_delete_session returned false");
        })
        .catch((e) => console.error("[project] db_permanent_delete_session failed:", e));
    }
    persist();
    return true;
  }

  /** 清空回收站中所有过期（超过指定天数）的会话。 */
  async function purgeExpiredTrash(retentionDays: number = 30): Promise<number> {
    if (!dbConnected.value) {
      // 本地回收站清理
      const cutoff = Date.now() - retentionDays * 24 * 3600 * 1000;
      const before = trashConversations.value.length;
      trashConversations.value = trashConversations.value.filter(
        (c) => !c.deletedAt || c.deletedAt > cutoff
      );
      return before - trashConversations.value.length;
    }
    try {
      const count = await invoke<number>("db_purge_expired_trash", { retentionDays });
      // 重新加载回收站以保持同步
      await loadTrashFromDb();
      return count;
    } catch (e) {
      console.error("[project] purge_expired_trash failed:", e);
      return 0;
    }
  }

  function switchTo(projectId: string, convId?: string) {
    activeProjectId.value = projectId;
    if (convId) {
      activeConversationId.value = convId;
    } else {
      const project = projects.value.find((p) => p.id === projectId);
      activeConversationId.value = project?.conversations[0]?.id ?? null;
    }
    persist();
  }

  /** Called by agent store after messages change — ensures persistence on every message */
  function persistMessages() {
    persist();
  }

  /** Update a conversation's title and optionally mark its source.
   *  Respects manual titles — won't overwrite if the current source is 'manual'. */
  function updateConversationTitle(
    projectId: string,
    convId: string,
    title: string,
    source: "auto" | "manual" = "auto"
  ): boolean {
    const project = projects.value.find((p) => p.id === projectId);
    if (!project) return false;
    const conv = project.conversations.find((c) => c.id === convId);
    if (!conv) return false;

    // Never overwrite a manually-set title with an auto-generated one
    if (source === "auto" && conv.titleSource === "manual") {
      return false;
    }

    conv.title = title;
    conv.titleSource = source;
    conv.updatedAt = Date.now();
    project.updatedAt = Date.now();

    if (dbConnected.value) {
      invoke("db_update_session_title", { id: convId, title })
        .catch((e) => console.error("[project] db_update_session_title failed:", e));
    }
    persist();
    return true;
  }

  /** Make a previously-hidden conversation visible in the sidebar.
   *  Called after the first AI response completes. */
  function revealConversation(projectId: string, convId: string): boolean {
    const project = projects.value.find((p) => p.id === projectId);
    if (!project) return false;
    const conv = project.conversations.find((c) => c.id === convId);
    if (!conv) return false;
    if (conv.visible !== false) return false; // already visible
    conv.visible = true;
    conv.updatedAt = Date.now();
    project.updatedAt = Date.now();

    if (dbConnected.value) {
      invoke<boolean>("db_reveal_session", { id: convId })
        .then((revealed) => {
          if (!revealed) {
            console.warn("[project] db_reveal_session returned false — session not found in SQLite");
          }
        })
        .catch((e) => console.error("[project] db_reveal_session failed:", e));
    }
    persist();
    return true;
  }

  // ── 消息持久化操作（流式持久化用）─────────────────────────

  /** 向 SQLite 插入新消息行，返回自增 id。 */
  async function dbAddMessage(sessionId: string, role: string, content: string, done: boolean, reasoning?: string, toolCallsJson?: string): Promise<number> {
    if (!dbConnected.value) return 0;
    try {
      const row: any = await invoke("db_add_message", {
        sessionId,
        role,
        content,
        done,
        reasoning: reasoning ?? null,
        toolCalls: toolCallsJson ?? null,
      });
      return row.id;
    } catch (e) {
      console.error("[project] db_add_message failed:", e);
      return 0;
    }
  }

  /** 更新 MySQL 中的消息字段（流式 delta 或完成标记）。仅 MySQL 模式调用。 */
  async function dbUpdateMessage(id: number, fields: Record<string, unknown>): Promise<void> {
    if (!dbConnected.value || !id) return;
    try {
      await invoke("db_update_message", { id, fieldsJson: JSON.stringify(fields) });
    } catch (e) {
      console.error("[project] db_update_message failed:", e);
    }
  }

  // ── 初始化 ──────────────────────────────────────────────

  // 尝试从 MySQL 加载（异步，不阻塞 store 创建）
  tryLoadFromMySql().then((loaded) => {
    if (loaded) {
      // 仅在没有项目时才创建默认项目，不预建会话 ——
      // 用户启动后看到空白对话界面，首次发送消息时由 ChatPanel 自动创建会话
      if (projects.value.length === 0) {
        const defaultProject = createProject("默认项目");
        activeProjectId.value = defaultProject.id;
      }
      validateState();
    } else {
      // localStorage 回退
      if (projects.value.length === 0) {
        const defaultProject = createProject("默认项目");
        activeProjectId.value = defaultProject.id;
      }
    }
  }).catch(() => {
    // 连不上 MySQL，回退到 localStorage
    if (projects.value.length === 0) {
      const defaultProject = createProject("默认项目");
      activeProjectId.value = defaultProject.id;
    }
  });

  return {
    projects,
    sortedProjects,
    activeProjectId,
    activeConversationId,
    activeProject,
    activeConversation,
    activeMessages,
    dbConnected,
    // 回收站
    trashConversations,
    trashExpanded,
    loadTrashFromDb,
    tryLoadFromMySql,
    createProject,
    importProject,
    deleteProject,
    pinProject,
    updateProjectName,
    createConversation,
    deleteConversation,
    restoreConversation,
    permanentDeleteConversation,
    purgeExpiredTrash,
    switchTo,
    persistMessages,
    updateConversationTitle,
    revealConversation,
    dbAddMessage,
    dbUpdateMessage,
  };
});
