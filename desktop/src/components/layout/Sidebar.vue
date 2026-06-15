<script setup lang="ts">
import { ref } from "vue";
import { useRouter } from "vue-router";
import { NButton, NTooltip, NInput, NScrollbar, NDropdown, useMessage } from "naive-ui";
import { useProjectStore } from "../../stores/project";
import { useThemeStore } from "../../stores/theme";
import { open as tauriOpen } from "@tauri-apps/plugin-dialog";

const router = useRouter();
const store = useProjectStore();
const themeStore = useThemeStore();
const message = useMessage();

const expanded = ref<Record<string, boolean>>({});
const headerHovered = ref(false);
const creatingProject = ref(false);
const newProjectName = ref("");
const creatingConvProjectId = ref<string | null>(null);
const newConvTitle = ref("");
const scrollRef = ref<InstanceType<typeof NScrollbar> | null>(null);

function isExpanded(projectId: string): boolean {
  return expanded.value[projectId] ?? false;
}

function toggleProject(projectId: string) {
  expanded.value[projectId] = !expanded.value[projectId];
  store.switchTo(projectId);
}

function selectConversation(projectId: string, convId: string) {
  expanded.value[projectId] = true;
  store.switchTo(projectId, convId);
}

function collapseAll() {
  expanded.value = {};
}

function startCreateProject() {
  creatingProject.value = true;
  newProjectName.value = "";
}

function finishCreateProject() {
  const name = newProjectName.value.trim();
  if (name) {
    const project = store.createProject(name);
    expanded.value[project.id] = true;
    store.switchTo(project.id);
  }
  creatingProject.value = false;
  newProjectName.value = "";
}

function cancelCreateProject() {
  creatingProject.value = false;
  newProjectName.value = "";
}

const projectActionOptions = [
  { label: "新建空白项目", key: "create" },
  { label: "使用本地文件", key: "import" },
];

function handleProjectAction(key: string) {
  if (key === "create") {
    startCreateProject();
  } else if (key === "import") {
    handleImportProject();
  }
}

async function handleImportProject() {
  try {
    const selected = await tauriOpen({ directory: true, multiple: false });
    if (selected && typeof selected === "string") {
      const name = selected.split(/[/\\]/).pop() || selected;
      const project = store.importProject(name, selected);
      expanded.value[project.id] = true;
      store.switchTo(project.id);
      message.success(`已导入项目: ${name}`);
    }
  } catch {
    fallbackImport();
  }
}

function fallbackImport() {
  const path = prompt("请输入项目路径:");
  if (path && path.trim()) {
    const name = path.trim().split(/[/\\]/).pop() || path.trim();
    const project = store.importProject(name, path.trim());
    expanded.value[project.id] = true;
    store.switchTo(project.id);
    message.success(`已导入项目: ${name}`);
  }
}

function startCreateConversation(projectId: string) {
  creatingConvProjectId.value = projectId;
  newConvTitle.value = "";
}

function finishCreateConversation() {
  const projectId = creatingConvProjectId.value;
  if (projectId) {
    const conv = store.createConversation(
      projectId,
      newConvTitle.value.trim() || undefined
    );
    store.switchTo(projectId, conv.id);
  }
  creatingConvProjectId.value = null;
  newConvTitle.value = "";
}

function handleDeleteConversation(projectId: string, convId: string) {
  store.deleteConversation(projectId, convId);
}
</script>

<template>
  <div class="sidebar" :class="{ dark: themeStore.isDark }">
    <!-- Logo -->
    <div class="sidebar-logo" @click="router.push('/')">
      <div class="logo-icon">
        <img src="/xuflow.png" alt="Xuflow" class="logo-img" />
      </div>
      <span class="logo-text">Xuflow</span>
    </div>

    <div class="sidebar-divider" />

    <!-- Project header -->
    <div
      class="project-header"
      @mouseenter="headerHovered = true"
      @mouseleave="headerHovered = false"
    >
      <span class="project-header-title">项目列表</span>
      <div class="project-header-actions">
        <NDropdown trigger="click" :options="projectActionOptions" @select="handleProjectAction">
          <NButton size="tiny" quaternary class="add-project-btn">
            <template #icon>
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <path
                  d="M2 4.5A1.5 1.5 0 013.5 3h3.172a1.5 1.5 0 011.06.44l.768.768a1.5 1.5 0 001.06.44H12.5A1.5 1.5 0 0114 6.148V12.5a1.5 1.5 0 01-1.5 1.5h-9A1.5 1.5 0 012 12.5V4.5z"
                  stroke="currentColor"
                  stroke-width="1.2"
                  fill="none"
                />
                <circle cx="12" cy="5" r="4" fill="currentColor" />
                <path
                  d="M10.5 5h3M12 3.5v3"
                  stroke="#fff"
                  stroke-width="1.2"
                  stroke-linecap="round"
                />
              </svg>
            </template>
          </NButton>
        </NDropdown>
        <NTooltip trigger="hover">
          <template #trigger>
            <NButton size="tiny" quaternary @click="collapseAll">
              <template #icon>
                <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
                  <path
                    d="M3 5l4 4 4-4"
                    stroke="currentColor"
                    stroke-width="1.4"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  />
                </svg>
              </template>
            </NButton>
          </template>
          全部收起
        </NTooltip>
      </div>
    </div>

    <!-- Inline create project input -->
    <div v-if="creatingProject" class="inline-create">
      <NInput
        v-model:value="newProjectName"
        size="small"
        placeholder="输入项目名称..."
        :autofocus="true"
        @keydown.enter="finishCreateProject"
        @keydown.escape="cancelCreateProject"
        @blur="finishCreateProject"
      />
    </div>

    <!-- Project list -->
    <NScrollbar ref="scrollRef" class="project-list-scroll">
      <div class="project-list">
        <div
          v-for="project in store.projects"
          :key="project.id"
          class="project-item"
          :class="{ active: store.activeProjectId === project.id }"
        >
          <!-- Project row -->
          <div class="project-row" @click="toggleProject(project.id)">
            <svg
              class="project-arrow"
              :class="{ expanded: isExpanded(project.id) }"
              width="12"
              height="12"
              viewBox="0 0 12 12"
              fill="none"
            >
              <path
                d="M4.5 3l3 3-3 3"
                stroke="currentColor"
                stroke-width="1.4"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" class="project-icon">
              <path
                d="M2 4.5A1.5 1.5 0 013.5 3h3.172a1.5 1.5 0 011.06.44l.768.768a1.5 1.5 0 001.06.44H12.5A1.5 1.5 0 0114 6.148V12.5a1.5 1.5 0 01-1.5 1.5h-9A1.5 1.5 0 012 12.5V4.5z"
                fill="currentColor"
                opacity="0.2"
              />
              <path
                d="M2 4.5A1.5 1.5 0 013.5 3h3.172a1.5 1.5 0 011.06.44l.768.768a1.5 1.5 0 001.06.44H12.5A1.5 1.5 0 0114 6.148V12.5a1.5 1.5 0 01-1.5 1.5h-9A1.5 1.5 0 012 12.5V4.5z"
                stroke="currentColor"
                stroke-width="1.2"
              />
            </svg>
            <span class="project-name">{{ project.name }}</span>
            <NButton
              v-show="isExpanded(project.id)"
              size="tiny"
              quaternary
              class="add-conv-btn"
              @click.stop="startCreateConversation(project.id)"
              title="新建会话"
            >
              <template #icon>
                <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                  <path
                    d="M6 2v8M2 6h8"
                    stroke="currentColor"
                    stroke-width="1.4"
                    stroke-linecap="round"
                  />
                </svg>
              </template>
            </NButton>
          </div>

          <!-- Inline create conversation input -->
          <div
            v-if="creatingConvProjectId === project.id"
            class="inline-create conv-create"
          >
            <NInput
              v-model:value="newConvTitle"
              size="small"
              placeholder="会话名称..."
              :autofocus="true"
              @keydown.enter="finishCreateConversation"
              @keydown.escape="
                creatingConvProjectId = null;
                newConvTitle = '';
              "
              @blur="finishCreateConversation"
            />
          </div>

          <!-- Conversation list -->
          <div v-if="isExpanded(project.id)" class="conversation-list">
            <div
              v-for="conv in project.conversations"
              :key="conv.id"
              class="conversation-item"
              :class="{ active: store.activeConversationId === conv.id }"
              @click="selectConversation(project.id, conv.id)"
            >
              <svg
                width="14"
                height="14"
                viewBox="0 0 14 14"
                fill="none"
                class="conv-icon"
              >
                <path
                  d="M2 3.5A1.5 1.5 0 013.5 2h7A1.5 1.5 0 0112 3.5v5a1.5 1.5 0 01-1.5 1.5H6l-2.5 2.5V10H3.5A1.5 1.5 0 012 8.5v-5z"
                  stroke="currentColor"
                  stroke-width="1.2"
                  stroke-linejoin="round"
                />
              </svg>
              <span class="conv-title">{{ conv.title }}</span>
              <NButton
                size="tiny"
                quaternary
                type="error"
                class="conv-delete-btn"
                @click.stop="handleDeleteConversation(project.id, conv.id)"
              >
                <template #icon>
                  <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                    <path
                      d="M3 3l6 6M9 3l-6 6"
                      stroke="currentColor"
                      stroke-width="1.4"
                      stroke-linecap="round"
                    />
                  </svg>
                </template>
              </NButton>
            </div>
            <div
              v-if="project.conversations.length === 0"
              class="conv-empty"
            >
              暂无会话
            </div>
          </div>
        </div>
      </div>
    </NScrollbar>

    <!-- Bottom -->
    <div class="sidebar-divider" />
    <div class="sidebar-bottom">
      <NButton text size="small" @click="router.push('/settings')" class="bottom-btn">
        <template #icon>
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path
              d="M8 10a2 2 0 100-4 2 2 0 000 4z"
              stroke="currentColor"
              stroke-width="1.4"
            />
            <path
              d="M13.5 8c0-.47-.06-.93-.17-1.37l1.52-1.19-1.5-2.6-1.87.59a5.52 5.52 0 00-2.36-1.37L8.97.5H6.03l-.15 1.57a5.52 5.52 0 00-2.36 1.37l-1.87-.59-1.5 2.6 1.52 1.19A5.47 5.47 0 001.5 8c0 .47.06.93.17 1.37l-1.52 1.19 1.5 2.6 1.87-.59c.72.56 1.52 1 2.36 1.37L5.93 15.5h2.94l.15-1.57a5.52 5.52 0 002.36-1.37l1.87.59 1.5-2.6-1.52-1.19c.11-.44.17-.9.17-1.37z"
              stroke="currentColor"
              stroke-width="1.2"
              stroke-linejoin="round"
            />
          </svg>
        </template>
        设置
      </NButton>
    </div>
  </div>
</template>

<style scoped>
.sidebar {
  width: 260px;
  min-width: 260px;
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: #fafafa;
  border-right: 1px solid rgba(0, 0, 0, 0.06);
  user-select: none;
  transition: background-color 0.3s ease, border-color 0.3s ease;
}

.sidebar.dark {
  background: #101014;
  border-right-color: rgba(255, 255, 255, 0.06);
}

/* Logo */
.sidebar-logo {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px 16px;
  cursor: pointer;
  flex-shrink: 0;
}

.logo-icon {
  display: flex;
  align-items: center;
  justify-content: center;
}

.logo-img {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  object-fit: contain;
}

.logo-text {
  font-size: 16px;
  font-weight: 700;
  letter-spacing: -0.3px;
}

.sidebar.dark .logo-text {
  color: #e2e8f0;
}

/* Divider */
.sidebar-divider {
  height: 1px;
  background: rgba(0, 0, 0, 0.06);
  flex-shrink: 0;
  margin: 0 12px;
}

.sidebar.dark .sidebar-divider {
  background: rgba(255, 255, 255, 0.06);
}

/* Inline create */
.inline-create {
  padding: 4px 12px;
  flex-shrink: 0;
}

.conv-create {
  padding: 4px 12px 4px 32px;
}

/* Project header */
.project-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 32px;
  padding: 0 16px;
  flex-shrink: 0;
}

.project-header-title {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: #94a3b8;
}

.sidebar.dark .project-header-title {
  color: #64748b;
}

.project-header-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}

.add-project-btn {
  color: #94a3b8;
}

.sidebar.dark .add-project-btn {
  color: #64748b;
}

/* Project list scroll */
.project-list-scroll {
  flex: 1;
  overflow-y: auto;
}

.project-list {
  padding: 4px 0;
}

/* Project item */
.project-item {
  cursor: pointer;
}

.project-item.active > .project-row {
  background: rgba(79, 70, 229, 0.08);
}

.sidebar.dark .project-item.active > .project-row {
  background: rgba(99, 102, 241, 0.15);
}

.project-row {
  display: flex;
  align-items: center;
  padding: 8px 12px;
  gap: 8px;
  font-size: 13px;
  transition: background-color 0.15s ease;
  border-radius: 6px;
  margin: 0 4px;
}

.project-row:hover {
  background: rgba(0, 0, 0, 0.04);
}

.sidebar.dark .project-row:hover {
  background: rgba(255, 255, 255, 0.04);
}

.project-arrow {
  flex-shrink: 0;
  color: #94a3b8;
  transition: transform 0.2s ease;
}

.project-arrow.expanded {
  transform: rotate(90deg);
}

.project-icon {
  flex-shrink: 0;
  color: #6366f1;
}

.sidebar.dark .project-icon {
  color: #818cf8;
}

.project-name {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-weight: 500;
}

.sidebar.dark .project-name {
  color: #e2e8f0;
}

.add-conv-btn {
  opacity: 0;
  flex-shrink: 0;
  transition: opacity 0.15s ease;
}

.project-row:hover .add-conv-btn {
  opacity: 1;
}

/* Conversation list */
.conversation-item {
  display: flex;
  align-items: center;
  padding: 6px 12px 6px 32px;
  gap: 8px;
  font-size: 13px;
  cursor: pointer;
  transition: background-color 0.15s ease;
  border-radius: 6px;
  margin: 1px 4px;
}

.conversation-item:hover {
  background: rgba(0, 0, 0, 0.04);
}

.sidebar.dark .conversation-item:hover {
  background: rgba(255, 255, 255, 0.04);
}

.conversation-item.active {
  background: rgba(79, 70, 229, 0.08);
}

.sidebar.dark .conversation-item.active {
  background: rgba(99, 102, 241, 0.15);
}

.conv-icon {
  flex-shrink: 0;
  color: #94a3b8;
}

.sidebar.dark .conv-icon {
  color: #64748b;
}

.conv-title {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  color: #475569;
}

.sidebar.dark .conv-title {
  color: #94a3b8;
}

.conv-delete-btn {
  opacity: 0;
  flex-shrink: 0;
  transition: opacity 0.15s ease;
}

.conversation-item:hover .conv-delete-btn {
  opacity: 1;
}

.conv-empty {
  padding: 6px 12px 6px 32px;
  font-size: 12px;
  color: #94a3b8;
  font-style: italic;
}

/* Bottom */
.sidebar-bottom {
  padding: 8px 12px;
  flex-shrink: 0;
}

.bottom-btn {
  width: 100%;
  justify-content: flex-start;
  gap: 8px;
}
</style>
