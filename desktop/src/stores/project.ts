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

    // 尝试从 localStorage 恢复活跃项目/会话 ID（MySQL 不存这个状态）
    const localState = loadFromLocalStorage();
    const activeProjectId = localState.activeProjectId ?? projects[0]?.id ?? null;
    const activeConversationId = localState.activeConversationId
      ?? projects[0]?.conversations.filter(c => c.visible !== false)[0]?.id
      ?? projects[0]?.conversations[0]?.id
      ?? null;

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
  const activeConversationId = ref<string | null>(saved.activeConversationId);

  /** SQLite 是否已就绪（启动即连接，始终为 true）。 */
  const dbConnected = ref(true);

  /** 尝试从 SQLite 加载数据。成功则替换 projects 并返回 true。 */
  async function tryLoadFromMySql(): Promise<boolean> {
    // SQLite 始终可用，直接加载
    try {
      const data = await loadFromMySql();
      projects.value = data.projects;
      activeProjectId.value = data.activeProjectId;
      activeConversationId.value = data.activeConversationId;
      console.log("[project] Loaded from SQLite:", projects.value.length, "projects");
      return true;
    } catch (e) {
      console.error("[project] Failed to load from SQLite, falling back to localStorage:", e);
      return false;
    }
  }

  /** Validate that saved IDs still point to real objects; fall back to first available.
   *  Skips invisible conversations when picking a fallback. */
  function validateState() {
    const project = projects.value.find((p) => p.id === activeProjectId.value);
    if (!project) {
      activeProjectId.value = projects.value[0]?.id ?? null;
    }
    const activeProj = projects.value.find((p) => p.id === activeProjectId.value);
    const conv = activeProj?.conversations.find((c) => c.id === activeConversationId.value);
    if (!conv) {
      const visibleConvs = activeProj?.conversations.filter((c) => c.visible !== false) ?? [];
      activeConversationId.value = visibleConvs[0]?.id ?? activeProj?.conversations[0]?.id ?? null;
    }
  }

  validateState();

  /** 持久化：SQLite 已由各方法实时写入，此处仅回退到 localStorage。 */
  function persist() {
    if (dbConnected.value) return; // SQLite 模式下不需要 localStorage
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
      invoke("db_create_project", { name, source: "local" })
        .then((row: any) => {
          // 用 MySQL 返回的 id 和 timestamp 覆盖本地值
          project.id = row.id;
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
      invoke("db_create_project", { name, source: "imported" })
        .then((row: any) => {
          project.id = row.id;
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
    projects.value.splice(idx, 1);
    if (activeProjectId.value === id) {
      activeProjectId.value = projects.value[0]?.id ?? null;
      activeConversationId.value =
        projects.value[0]?.conversations[0]?.id ?? null;
    }

    if (dbConnected.value) {
      invoke("db_delete_project", { id })
        .catch((e) => console.error("[project] db_delete_project failed:", e));
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
      invoke("db_create_session", {
        projectId,
        title: conv.title,
        titleSource: conv.titleSource,
        visible,
      })
        .then((row: any) => {
          conv.id = row.id;
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
    project.conversations.splice(idx, 1);
    project.updatedAt = Date.now();
    if (activeConversationId.value === convId) {
      activeConversationId.value = project.conversations[0]?.id ?? null;
    }

    if (dbConnected.value) {
      invoke("db_delete_session", { id: convId })
        .catch((e) => console.error("[project] db_delete_session failed:", e));
    }
    persist();
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
      invoke("db_reveal_session", { id: convId })
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
      // MySQL 加载成功后的后处理
      if (projects.value.length === 0) {
        const defaultProject = createProject("默认项目");
        const defaultConv = createConversation(defaultProject.id, "默认会话");
        activeProjectId.value = defaultProject.id;
        activeConversationId.value = defaultConv.id;
      }
      validateState();
    } else {
      // localStorage 回退
      if (projects.value.length === 0) {
        const defaultProject = createProject("默认项目");
        const defaultConv = createConversation(defaultProject.id, "默认会话");
        activeProjectId.value = defaultProject.id;
        activeConversationId.value = defaultConv.id;
      }
    }
  }).catch(() => {
    // 连不上 MySQL，回退到 localStorage
    if (projects.value.length === 0) {
      const defaultProject = createProject("默认项目");
      const defaultConv = createConversation(defaultProject.id, "默认会话");
      activeProjectId.value = defaultProject.id;
      activeConversationId.value = defaultConv.id;
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
    tryLoadFromMySql,
    createProject,
    importProject,
    deleteProject,
    pinProject,
    updateProjectName,
    createConversation,
    deleteConversation,
    switchTo,
    persistMessages,
    updateConversationTitle,
    revealConversation,
    dbAddMessage,
    dbUpdateMessage,
  };
});
