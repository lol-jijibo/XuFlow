<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
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

// ── 右键上下文菜单 ──────────────────────────────────────────
// 鼠标右击项目行时弹出操作列表（删除、置顶等），点击其他区域自动关闭。

const contextMenu = ref({
  show: false,
  x: 0,
  y: 0,
  projectId: null as string | null,
});

/** 打开右键菜单：记录鼠标位置和目标项目 ID，自动调整避免溢出屏幕边缘。 */
function onProjectContextMenu(e: MouseEvent, projectId: string) {
  e.preventDefault();
  e.stopPropagation();
  // 菜单预估尺寸：宽约 150px，每项高约 32px（置顶+删除共 2 项约 72px）
  const menuW = 150;
  const menuH = 80;
  let x = e.clientX;
  let y = e.clientY;
  if (x + menuW > window.innerWidth) x = window.innerWidth - menuW - 8;
  if (y + menuH > window.innerHeight) y = window.innerHeight - menuH - 8;
  contextMenu.value = { show: true, x, y, projectId };
}

/** 关闭右键菜单。 */
function closeContextMenu() {
  contextMenu.value.show = false;
}

/** 全局点击时关闭右键菜单（点菜单自身除外）。 */
function onGlobalClick(_e: MouseEvent) {
  if (contextMenu.value.show) {
    closeContextMenu();
  }
}

/** 处理右键菜单选项：删除项目 / 置顶项目。 */
function handleContextMenuSelect(key: string) {
  const projectId = contextMenu.value.projectId;
  closeContextMenu();
  if (!projectId) return;

  if (key === "delete") {
    const project = store.projects.find((p) => p.id === projectId);
    if (project) {
      store.deleteProject(projectId);
      message.success(`已删除项目: ${project.name}`);
    }
  } else if (key === "pin") {
    const project = store.projects.find((p) => p.id === projectId);
    const pinned = store.pinProject(projectId);
    if (project) {
      message.success(pinned ? `已置顶: ${project.name}` : `已取消置顶: ${project.name}`);
    }
  }
}

// 右键菜单选项：根据当前项目置顶状态动态生成。
const contextMenuOptions = computed(() => {
  const project = contextMenu.value.projectId
    ? store.projects.find((p) => p.id === contextMenu.value.projectId)
    : null;
  return [
    {
      label: project?.pinned ? "取消置顶" : "置顶项目",
      key: "pin",
    },
    {
      label: "删除项目",
      key: "delete",
    },
  ];
});

onMounted(() => {
  document.addEventListener("click", onGlobalClick);
});

onUnmounted(() => {
  document.removeEventListener("click", onGlobalClick);
});

const creatingProject = ref(false);
const newProjectName = ref("");
const scrollRef = ref<InstanceType<typeof NScrollbar> | null>(null);

// 重命名状态：项目名和会话名内联编辑
const renamingProjectId = ref<string | null>(null);
const renameProjectName = ref("");
const renamingConvInfo = ref<{ projectId: string; convId: string } | null>(null);
const renameConvTitle = ref("");

/** Format a timestamp as a short relative label in Chinese. */
function formatRelativeTime(ts: number): string {
  const diff = Date.now() - ts;
  const sec = Math.floor(diff / 1000);
  if (sec < 60) return "刚刚";
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}分钟`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}小时`;
  const day = Math.floor(hr / 24);
  if (day < 7) return `${day}天`;
  const wk = Math.floor(day / 7);
  if (wk < 4) return `${wk}周`;
  const mo = Math.floor(day / 30);
  return `${mo}月`;
}

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

// ── 项目名重命名（双击触发内联编辑）────────────────────────────
// 进入编辑模式前先提交另一个类型的编辑，保证最多一个输入框活跃

function startRenameProject(projectId: string) {
  if (renamingProjectId.value) finishRenameProject();
  if (renamingConvInfo.value) finishRenameConversation();
  const project = store.projects.find((p) => p.id === projectId);
  if (!project) return;
  renamingProjectId.value = projectId;
  renameProjectName.value = project.name;
}

function finishRenameProject() {
  const id = renamingProjectId.value;
  if (!id) return;
  const name = renameProjectName.value.trim();
  if (name) store.updateProjectName(id, name);
  renamingProjectId.value = null;
  renameProjectName.value = "";
}

function cancelRenameProject() {
  renamingProjectId.value = null;
  renameProjectName.value = "";
}

// ── 会话名重命名（双击触发内联编辑）────────────────────────────

function startRenameConversation(projectId: string, convId: string) {
  if (renamingProjectId.value) finishRenameProject();
  if (renamingConvInfo.value) finishRenameConversation();
  const project = store.projects.find((p) => p.id === projectId);
  const conv = project?.conversations.find((c) => c.id === convId);
  if (!conv) return;
  renamingConvInfo.value = { projectId, convId };
  renameConvTitle.value = conv.title;
}

function finishRenameConversation() {
  const info = renamingConvInfo.value;
  if (!info) return;
  const title = renameConvTitle.value.trim();
  if (title) store.updateConversationTitle(info.projectId, info.convId, title, "manual");
  renamingConvInfo.value = null;
  renameConvTitle.value = "";
}

function cancelRenameConversation() {
  renamingConvInfo.value = null;
  renameConvTitle.value = "";
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

// 点击项目旁的 + 号，直接在该项目下新建空白会话界面，不弹出命名输入框
function startCreateConversation(projectId: string) {
  expanded.value[projectId] = true;
  store.activeProjectId = projectId;
  store.activeConversationId = null;
}

function handleDeleteConversation(projectId: string, convId: string) {
  store.deleteConversation(projectId, convId);
}

/** 清空当前活跃会话，切换到空白对话状态。
 *  不立即创建会话 —— 等用户发送第一条消息时再按需创建。
 *  会话标题在 AI 回复完成后自动提炼，避免侧边栏出现 "新会话 + 数字序号"。 */
function handleNewConversation() {
  const projectId = store.activeProjectId;
  if (!projectId) return;
  expanded.value[projectId] = true;
  store.activeConversationId = null;
}
</script>

<template>
  <div class="sidebar" :class="{ dark: themeStore.isDark }">
    <!-- 新会话按钮（固定在最上方，不受项目展开/折叠影响） -->
    <div class="new-conv-section">
      <NButton
        quaternary
        class="new-conv-btn"
        :disabled="!store.activeProjectId"
        @click="handleNewConversation"
      >
        <template #icon>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round">
            <circle cx="12" cy="12" r="9" />
            <line x1="12" y1="8" x2="12" y2="16" />
            <line x1="8" y1="12" x2="16" y2="12" />
          </svg>
        </template>
        新会话
      </NButton>
    </div>

    <!-- 项目标题（位于新会话下方、项目名上方） -->
    <div class="project-header">
      <span class="project-header-title">项目</span>
      <div class="project-header-actions">
        <NDropdown trigger="click" :options="projectActionOptions" @select="handleProjectAction">
          <NButton size="tiny" quaternary class="add-project-btn">
            <template #icon>
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
            </template>
          </NButton>
        </NDropdown>
        <NTooltip trigger="hover">
          <template #trigger>
            <NButton size="tiny" quaternary @click="collapseAll">
              <template #icon>
                <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
                  <path d="M3 5l4 4 4-4" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>
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
          v-for="project in store.sortedProjects"
          :key="project.id"
          class="project-item"
        >
          <!-- Project row -->
          <div class="project-row" @click="toggleProject(project.id)" @contextmenu="onProjectContextMenu($event, project.id)">
            <!-- Chevron -->
            <svg
              class="project-chevron"
              :class="{ expanded: isExpanded(project.id) }"
              width="14"
              height="14"
              viewBox="0 0 14 14"
              fill="none"
            >
              <path d="M5 3l4 4-4 4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            <!-- 闭合文件夹：仅线框轮廓，表示未展开 -->
            <svg v-if="!isExpanded(project.id)" width="16" height="16" viewBox="0 0 16 16" fill="none" class="project-icon">
              <path d="M2 4.5A1.5 1.5 0 013.5 3h2.672a1.5 1.5 0 011.06.44l.768.768a1.5 1.5 0 001.06.44H12.5A1.5 1.5 0 0114 6.148V12.5a1.5 1.5 0 01-1.5 1.5h-9A1.5 1.5 0 012 12.5V4.5z" stroke="currentColor" stroke-width="1.25" fill="none"/>
            </svg>
            <!-- 打开文件夹：半透明填充 + 内部横线 + 翻盖折页，明确表示已展开 -->
            <svg v-else width="16" height="16" viewBox="0 0 16 16" fill="none" class="project-icon project-icon--open">
              <!-- 主体轮廓 + 浅填充暗示内容可见 -->
              <path d="M2 4.5A1.5 1.5 0 013.5 3h2.672a1.5 1.5 0 011.06.44l.768.768a1.5 1.5 0 001.06.44H12.5A1.5 1.5 0 0114 6.148V12.5a1.5 1.5 0 01-1.5 1.5h-9A1.5 1.5 0 012 12.5V4.5z" stroke="currentColor" stroke-width="1.25" fill="currentColor" fill-opacity="0.12"/>
              <!-- 内部横线：暗示文件夹内容已可见 -->
              <line x1="5" y1="8.5" x2="11" y2="8.5" stroke="currentColor" stroke-width="1" stroke-linecap="round" opacity="0.45"/>
              <!-- 翻盖折页：左上角翻开的视觉线索 -->
              <path d="M3.5 4l1.5-2h2.5" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round" opacity="0.6"/>
            </svg>
            <!-- 置顶图标 -->
            <svg v-if="project.pinned" width="12" height="12" viewBox="0 0 14 14" fill="none" class="pin-indicator" title="已置顶">
              <path d="M9.5 2L8.3 3.2l.5.5L10 4.8l.5-.5L12 5.5v-6L8.5 3l.5.5-2 2-1.5-1.5L4 5.5l4 4 1.5-1.5-1.5-1.5 2-2z" fill="currentColor" stroke="currentColor" stroke-width="0.8" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            <!-- 项目名：双击进入内联重命名，Enter/blur 确认，Escape 取消 -->
            <span
              v-if="renamingProjectId !== project.id"
              class="project-name"
              @dblclick.stop="startRenameProject(project.id)"
              title="双击重命名"
            >{{ project.name }}</span>
            <NInput
              v-else
              v-model:value="renameProjectName"
              size="small"
              :autofocus="true"
              placeholder="项目名称"
              @keydown.enter="finishRenameProject"
              @keydown.escape="cancelRenameProject"
              @blur="finishRenameProject"
            />
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
                  <path d="M6 2v8M2 6h8" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
                </svg>
              </template>
            </NButton>
          </div>

          <!-- Conversation list -->
          <div v-if="isExpanded(project.id)" class="conversation-list">
            <div
              v-for="conv in project.conversations.filter(c => c.visible !== false)"
              :key="conv.id"
              class="conversation-item"
              :class="{ active: store.activeConversationId === conv.id }"
              @click="selectConversation(project.id, conv.id)"
            >
              <!-- Chat bubble icon — wireframe -->
              <svg width="15" height="15" viewBox="0 0 15 15" fill="none" class="conv-icon">
                <path d="M2.5 3.5a1 1 0 011-1h8a1 1 0 011 1v5.5a1 1 0 01-1 1H7.5L5 12.5V10H3.5a1 1 0 01-1-1v-5.5z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>
              </svg>
              <!-- 会话名：双击进入内联重命名，Enter/blur 确认，Escape 取消 -->
              <span
                v-if="!renamingConvInfo || renamingConvInfo.convId !== conv.id"
                class="conv-title"
                @dblclick.stop="startRenameConversation(project.id, conv.id)"
                title="双击重命名"
              >{{ conv.title }}</span>
              <NInput
                v-else
                v-model:value="renameConvTitle"
                size="small"
                :autofocus="true"
                placeholder="会话名称"
                @keydown.enter="finishRenameConversation"
                @keydown.escape="cancelRenameConversation"
                @blur="finishRenameConversation"
              />
              <span class="conv-time">{{ formatRelativeTime(conv.updatedAt) }}</span>
              <NButton
                size="tiny"
                quaternary
                type="error"
                class="conv-delete-btn"
                @click.stop="handleDeleteConversation(project.id, conv.id)"
              >
                <template #icon>
                  <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                    <path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
                  </svg>
                </template>
              </NButton>
            </div>
            <div
              v-if="project.conversations.filter(c => c.visible !== false).length === 0"
              class="conv-empty"
            >
              暂无会话
            </div>
          </div>
        </div>
      </div>
    </NScrollbar>

    <!-- Bottom — global settings, clearly separated -->
    <div class="sidebar-divider sidebar-divider--bottom" />
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

    <!-- 右键上下文菜单：固定定位在鼠标位置，点击外部自动关闭 -->
    <Teleport to="body">
      <div
        v-if="contextMenu.show"
        class="context-menu-backdrop"
        @click="closeContextMenu"
        @contextmenu.prevent="closeContextMenu"
      >
        <div
          class="context-menu"
          :class="{ dark: themeStore.isDark }"
          :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }"
        >
          <div
            v-for="opt in contextMenuOptions"
            :key="opt.key"
            class="context-menu-item"
            :class="{ danger: opt.key === 'delete' }"
            @click.stop="handleContextMenuSelect(opt.key)"
          >
            <!-- 置顶图标：图钉 -->
            <svg v-if="opt.key === 'pin'" width="14" height="14" viewBox="0 0 14 14" fill="none" class="context-menu-icon">
              <path d="M9.5 2L8.3 3.2l.5.5L10 4.8l.5-.5L12 5.5v-6L8.5 3l.5.5-2 2-1.5-1.5L4 5.5l4 4 1.5-1.5-1.5-1.5 2-2z" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            <!-- 删除图标：叉号 -->
            <svg v-else-if="opt.key === 'delete'" width="14" height="14" viewBox="0 0 14 14" fill="none" class="context-menu-icon">
              <path d="M3 3l8 8M11 3l-8 8" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
            </svg>
            <span>{{ opt.label }}</span>
          </div>
        </div>
      </div>
    </Teleport>
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
  background: #1a1a20;
  border-right-color: rgba(255, 255, 255, 0.06);
}

/* Shared divider */
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

/* New conversation button */
.new-conv-section {
  padding: 8px 10px;
  flex-shrink: 0;
}

.new-conv-btn {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: flex-start;
  gap: 2px;
  font-size: 14px;
  font-weight: 500;
  color: #6b7280;
  border-radius: 5px;
  padding: 8px 10px;
  transition: background 0.12s ease, color 0.12s ease;
}

.new-conv-btn:hover {
  background: rgba(0, 0, 0, 0.04);
  color: #374151;
}

.sidebar.dark .new-conv-btn {
  color: #ffffff;
}

.sidebar.dark .new-conv-btn:hover {
  background: rgba(255, 255, 255, 0.04);
  color: #e4e4e7;
}

/* Project header */
.project-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 34px;
  padding: 0 14px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
  flex-shrink: 0;
}

.sidebar.dark .project-header {
  border-bottom-color: rgba(255, 255, 255, 0.08);
}

.project-header-title {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: #94a3b8;
}

.sidebar.dark .project-header-title {
  color: #9ca3af;
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
  color: #ffffff;
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

/* Project row — folder level, tight left padding */
.project-row {
  display: flex;
  align-items: center;
  padding: 7px 10px 7px 6px;
  gap: 6px;
  font-size: 13px;
  transition: background-color 0.12s ease;
  border-radius: 5px;
  margin: 1px 6px;
}

.project-row:hover {
  background: rgba(0, 0, 0, 0.04);
}

.sidebar.dark .project-row:hover {
  background: rgba(255, 255, 255, 0.04);
}

/* Chevron arrow */
.project-chevron {
  flex-shrink: 0;
  color: #9ca3af;
  transition: transform 0.15s ease;
}

.project-chevron.expanded {
  transform: rotate(90deg);
}

.sidebar.dark .project-chevron {
  color: #ffffff;
}

/* Folder icon */
.project-icon {
  flex-shrink: 0;
  color: #9ca3af;
  transition: color 0.15s ease;
}

.sidebar.dark .project-icon {
  color: #ffffff;
}

/* 打开状态的文件夹图标颜色加深，增强视觉区分 */
.project-icon--open {
  color: #6b7280;
}

.sidebar.dark .project-icon--open {
  color: #d1d5db;
}

/* 置顶指示图标 */
.pin-indicator {
  flex-shrink: 0;
  color: #f59e0b;
  margin-right: -2px;
}

.sidebar.dark .pin-indicator {
  color: #fbbf24;
}

/* Project name */
.project-name {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-weight: 500;
  font-size: 13px;
  color: #374151;
}

.sidebar.dark .project-name {
  color: #ffffff;
}

.add-conv-btn {
  opacity: 0;
  flex-shrink: 0;
  transition: opacity 0.12s ease;
}

.project-row:hover .add-conv-btn {
  opacity: 1;
}

/* Conversation list */
.conversation-list {
  padding-top: 4px;
}

/* ── Conversation list ──────────────────────── */

.conversation-item {
  display: flex;
  align-items: center;
  padding: 7px 10px 7px 34px;   /* pl-34 = indent under folder */
  gap: 8px;
  font-size: 13px;
  cursor: pointer;
  transition: background-color 0.12s ease;
  border-radius: 4px;
  margin: 1px 6px;
}

.conversation-item:hover {
  background: rgba(0, 0, 0, 0.04);
}

.sidebar.dark .conversation-item:hover {
  background: rgba(255, 255, 255, 0.04);
}

/* Active block for selected conversation — subtle */
.conversation-item.active {
  background: rgba(0, 0, 0, 0.05);
}

.sidebar.dark .conversation-item.active {
  background: rgba(255, 255, 255, 0.06);
}

/* Chat bubble icon */
.conv-icon {
  flex-shrink: 0;
  color: #9ca3af;
  transition: color 0.15s ease;
}

.sidebar.dark .conv-icon {
  color: #ffffff;
}

.conversation-item.active .conv-icon {
  color: #6b7280;
}

.sidebar.dark .conversation-item.active .conv-icon {
  color: #ffffff;
}

/* Conversation title — 无衬线极客风，Inter / SF Pro 优先，抗锯齿 */
.conv-title {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-family: "Inter", "SF Pro Text", "SF Pro Display", -apple-system, BlinkMacSystemFont, "Segoe UI", "Helvetica Neue", sans-serif;
  font-size: 14px;
  font-weight: 450;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  color: #374151;
}

.sidebar.dark .conv-title {
  color: #e4e4e7;
}

.conversation-item.active .conv-title {
  color: #111827;
  font-weight: 470;
}

.sidebar.dark .conversation-item.active .conv-title {
  color: #ffffff;
  font-weight: 470;
}

/* Relative timestamp */
.conv-time {
  flex-shrink: 0;
  font-size: 11px;
  color: #9ca3af;
  white-space: nowrap;
  margin-left: auto;
}

.sidebar.dark .conv-time {
  color: #ffffff;
}

.conversation-item.active .conv-time {
  color: #94a3b8;
}

.sidebar.dark .conversation-item.active .conv-time {
  color: #ffffff;
}

/* Delete button */
.conv-delete-btn {
  opacity: 0;
  flex-shrink: 0;
  transition: opacity 0.12s ease;
  margin-left: 2px;
}

.conversation-item:hover .conv-delete-btn {
  opacity: 1;
}

/* Empty state */
.conv-empty {
  padding: 8px 12px 8px 34px;
  font-size: 12px;
  color: #9ca3af;
  font-style: italic;
}

.sidebar.dark .conv-empty {
  color: #ffffff;
}

/* Bottom divider — separates global settings from project list */
.sidebar-divider--bottom {
  margin: 6px 12px 8px;
  background: rgba(0, 0, 0, 0.08);
  height: 1px;
}

.sidebar.dark .sidebar-divider--bottom {
  background: rgba(255, 255, 255, 0.08);
}

/* Bottom — global settings */
.sidebar-bottom {
  padding: 6px 10px 10px;
  flex-shrink: 0;
}

.bottom-btn {
  width: 100%;
  justify-content: flex-start;
  gap: 8px;
  font-size: 13px;
  color: #6b7280;
  border-radius: 5px;
  padding: 6px 8px;
  transition: background 0.12s ease, color 0.12s ease;
}

.bottom-btn:hover {
  background: rgba(0, 0, 0, 0.04);
  color: #374151;
}

.sidebar.dark .bottom-btn {
  color: #ffffff;
}

.sidebar.dark .bottom-btn:hover {
  background: rgba(255, 255, 255, 0.04);
  color: #e4e4e7;
}

/* ── 右键上下文菜单 ──────────────────────────────── */

/* 全屏透明遮罩：点击任意位置关闭菜单 */
.context-menu-backdrop {
  position: fixed;
  inset: 0;
  z-index: 9999;
}

/* 菜单面板：浮于遮罩之上，跟随鼠标位置 */
.context-menu {
  position: fixed;
  min-width: 140px;
  background: #ffffff;
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.1), 0 1px 4px rgba(0, 0, 0, 0.06);
  padding: 4px;
  z-index: 10000;
  overflow: hidden;
}

.context-menu.dark {
  background: #25252b;
  border-color: rgba(255, 255, 255, 0.08);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4), 0 1px 4px rgba(0, 0, 0, 0.2);
}

/* 菜单项 */
.context-menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 10px;
  font-size: 13px;
  color: #374151;
  border-radius: 5px;
  cursor: pointer;
  transition: background-color 0.12s ease;
  white-space: nowrap;
}

.context-menu-item:hover {
  background: rgba(0, 0, 0, 0.04);
}

.context-menu.dark .context-menu-item {
  color: #e4e4e7;
}

.context-menu.dark .context-menu-item:hover {
  background: rgba(255, 255, 255, 0.06);
}

/* 删除项使用红色高亮 */
.context-menu-item.danger {
  color: #ef4444;
}

.context-menu-item.danger:hover {
  background: rgba(239, 68, 68, 0.08);
}

.context-menu.dark .context-menu-item.danger {
  color: #f87171;
}

.context-menu.dark .context-menu-item.danger:hover {
  background: rgba(248, 113, 113, 0.12);
}

/* 菜单图标 */
.context-menu-icon {
  flex-shrink: 0;
}
</style>
