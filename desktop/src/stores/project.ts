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

export const useProjectStore = defineStore("project", () => {
  const projects = ref<Project[]>([]);
  const activeProjectId = ref<string | null>(null);
  const activeConversationId = ref<string | null>(null);

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
  }

  function switchTo(projectId: string, convId?: string) {
    activeProjectId.value = projectId;
    if (convId) {
      activeConversationId.value = convId;
    } else {
      const project = projects.value.find((p) => p.id === projectId);
      activeConversationId.value = project?.conversations[0]?.id ?? null;
    }
  }

  // Initialize: default project + default conversation
  if (projects.value.length === 0) {
    const defaultProject = createProject("默认项目");
    const defaultConv = createConversation(defaultProject.id, "默认会话");
    activeProjectId.value = defaultProject.id;
    activeConversationId.value = defaultConv.id;
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
  };
});
