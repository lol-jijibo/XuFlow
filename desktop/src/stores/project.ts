import { defineStore } from "pinia";
import { ref, computed } from "vue";

export interface ChatMessage {
  role: "user" | "assistant" | "system";
  content: string;
  done: boolean;
}

export interface Conversation {
  id: string;
  title: string;
  messages: ChatMessage[];
  createdAt: number;
  updatedAt: number;
}

export interface Project {
  id: string;
  name: string;
  path?: string;
  source: "local" | "imported";
  conversations: Conversation[];
  createdAt: number;
  updatedAt: number;
}

let nextId = 1;
function uid(): string {
  return `${Date.now()}-${nextId++}`;
}

const STORAGE_KEY = "xuflow-projects";

function loadState(): { projects: Project[]; activeProjectId: string | null; activeConversationId: string | null } {
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

function saveState(
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

export const useProjectStore = defineStore("project", () => {
  const saved = loadState();
  const projects = ref<Project[]>(saved.projects);
  const activeProjectId = ref<string | null>(saved.activeProjectId);
  const activeConversationId = ref<string | null>(saved.activeConversationId);

  /** Validate that saved IDs still point to real objects; fall back to first available */
  function validateState() {
    // Validate active project
    const project = projects.value.find((p) => p.id === activeProjectId.value);
    if (!project) {
      activeProjectId.value = projects.value[0]?.id ?? null;
    }
    // Validate active conversation within the active project
    const activeProj = projects.value.find((p) => p.id === activeProjectId.value);
    const conv = activeProj?.conversations.find((c) => c.id === activeConversationId.value);
    if (!conv) {
      activeConversationId.value = activeProj?.conversations[0]?.id ?? null;
    }
  }

  validateState();

  /** Persist current state to localStorage after every mutation */
  function persist() {
    saveState(projects.value, activeProjectId.value, activeConversationId.value);
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
    persist();
  }

  function createConversation(projectId: string, title?: string): Conversation {
    const project = projects.value.find((p) => p.id === projectId);
    if (!project) throw new Error(`Project ${projectId} not found`);
    const conv: Conversation = {
      id: uid(),
      title: title || `新会话 ${project.conversations.length + 1}`,
      messages: [],
      createdAt: Date.now(),
      updatedAt: Date.now(),
    };
    project.conversations.push(conv);
    project.updatedAt = Date.now();
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

  // Initialize: default project + default conversation (only on first launch)
  if (projects.value.length === 0) {
    const defaultProject = createProject("默认项目");
    const defaultConv = createConversation(defaultProject.id, "默认会话");
    activeProjectId.value = defaultProject.id;
    activeConversationId.value = defaultConv.id;
    // persist() already called inside createProject + createConversation
  }

  return {
    projects,
    activeProjectId,
    activeConversationId,
    activeProject,
    activeConversation,
    activeMessages,
    createProject,
    importProject,
    deleteProject,
    createConversation,
    deleteConversation,
    switchTo,
    persistMessages,
  };
});
