<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { useRouter } from "vue-router";
import { NButton, NTooltip, NInput, NScrollbar, NDropdown, NModal, useMessage } from "naive-ui";
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
// 支持项目行右键（重命名、置顶、删除）与会话行右键（重命名、删除）。

type ContextMenuTarget =
  | { type: "project"; projectId: string }
  | { type: "conversation"; projectId: string; convId: string };

const contextMenu = ref({
  show: false,
  x: 0,
  y: 0,
  target: null as ContextMenuTarget | null,
});

/** 项目行右键菜单。 */
function onProjectContextMenu(e: MouseEvent, projectId: string) {
  e.preventDefault();
  e.stopPropagation();
  const menuH = 104;
  showContextMenu(e, menuH, { type: "project", projectId });
}

/** 会话行右键菜单。 */
function onConvContextMenu(e: MouseEvent, projectId: string, convId: string) {
  e.preventDefault();
  e.stopPropagation();
  const menuH = 72; // 重命名 + 删除，2 项
  showContextMenu(e, menuH, { type: "conversation", projectId, convId });
}

function showContextMenu(e: MouseEvent, menuH: number, target: ContextMenuTarget) {
  const menuW = 150;
  let x = e.clientX;
  let y = e.clientY;
  if (x + menuW > window.innerWidth) x = window.innerWidth - menuW - 8;
  if (y + menuH > window.innerHeight) y = window.innerHeight - menuH - 8;
  contextMenu.value = { show: true, x, y, target };
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

/** 处理右键菜单选项。 */
function handleContextMenuSelect(key: string) {
  const target = contextMenu.value.target;
  closeContextMenu();
  if (!target) return;

  if (target.type === "project") {
    if (key === "rename") {
      openRenameDialog(target.projectId);
    } else if (key === "delete") {
      const project = store.projects.find((p) => p.id === target.projectId);
      if (project) {
        store.deleteProject(target.projectId);
        message.success(`已删除项目: ${project.name}`);
      }
    } else if (key === "pin") {
      const project = store.projects.find((p) => p.id === target.projectId);
      const pinned = store.pinProject(target.projectId);
      if (project) {
        message.success(pinned ? `已置顶: ${project.name}` : `已取消置顶: ${project.name}`);
      }
    }
  } else if (target.type === "conversation") {
    if (key === "rename") {
      startRenameConversation(target.projectId, target.convId);
    } else if (key === "delete") {
      const project = store.projects.find((p) => p.id === target.projectId);
      const conv = project?.conversations.find((c) => c.id === target.convId);
      if (conv) {
        store.deleteConversation(target.projectId, target.convId);
        message.success(`已删除会话: ${conv.title}`);
      }
    }
  }
}

// 右键菜单选项：根据目标类型显示不同的操作列表。
const contextMenuOptions = computed(() => {
  const target = contextMenu.value.target;
  if (!target) return [];

  if (target.type === "project") {
    const project = store.projects.find((p) => p.id === target.projectId);
    return [
      { label: "重命名项目", key: "rename" },
      { label: project?.pinned ? "取消置顶" : "置顶项目", key: "pin" },
      { label: "删除项目", key: "delete" },
    ];
  }

  // conversation
  return [
    { label: "重命名会话", key: "rename" },
    { label: "删除会话", key: "delete" },
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

// 重命名状态：项目名使用居中弹窗编辑，会话名使用内联编辑
const showRenameDialog = ref(false);
const renameDialogProjectId = ref<string | null>(null);
const renameDialogName = ref("");
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

// ── 项目名重命名（右键菜单触发居中弹窗编辑）────────────────────────────
// 弹窗居中显示，输入新名称后按 Enter 或点击确认按钮保存，Esc 或点击遮罩取消。

/** 打开项目重命名弹窗：预填当前项目名，清空上一次的残留状态。 */
function openRenameDialog(projectId: string) {
  if (renamingConvInfo.value) finishRenameConversation();
  const project = store.projects.find((p) => p.id === projectId);
  if (!project) return;
  renameDialogProjectId.value = projectId;
  renameDialogName.value = project.name;
  showRenameDialog.value = true;
}

/** 确认重命名：非空名称才提交更新，否则静默关闭。 */
function confirmRenameDialog() {
  const id = renameDialogProjectId.value;
  if (!id) return;
  const name = renameDialogName.value.trim();
  if (name) store.updateProjectName(id, name);
  closeRenameDialog();
}

/** 关闭重命名弹窗：清空状态，丢弃未确认的输入。 */
function closeRenameDialog() {
  showRenameDialog.value = false;
  renameDialogProjectId.value = null;
  renameDialogName.value = "";
}

// ── 会话名重命名（双击触发内联编辑）────────────────────────────

function startRenameConversation(projectId: string, convId: string) {
  // 项目重命名使用弹窗，无需检查内联状态
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

/** 清空当前活跃会话，切换到空白对话状态。
 *  不立即创建会话 —— 等用户发送第一条消息时再按需创建。
 *  会话标题在 AI 回复完成后自动提炼，避免侧边栏出现 "新会话 + 数字序号"。
 *
 *  如果当前会话是隐藏状态（AI 回复尚未完成），在切换前先将其抢救到侧边栏：
 *  提取第一条用户提示词作为标题、标记为可见，但不中断后端 AI 生成。
 *  这样用户点击新会话后可以立即开始新的对话，而之前的会话继续在后台
 *  生成并保留在侧边栏中，随时可切换回去查看。 */
function handleNewConversation() {
  const projectId = store.activeProjectId;
  if (!projectId) return;

  // 抢救隐藏会话：提取用户第一条提示词为标题，显示到侧边栏，但不停止 AI 生成
  const currentConv = store.activeConversation;
  if (currentConv && currentConv.visible === false && currentConv.messages.length > 0) {
    // 用第一条用户消息提炼会话标题
    if (!currentConv.title) {
      const firstUserMsg = currentConv.messages.find((m) => m.role === "user");
      if (firstUserMsg) {
        const rawTitle = firstUserMsg.content.trim();
        const fallbackTitle = rawTitle.length > 50 ? rawTitle.slice(0, 49) + "…" : rawTitle;
        store.updateConversationTitle(projectId, currentConv.id, fallbackTitle, "auto");
      }
    }

    // 显示到侧边栏（AI 回复完成后 agent:done 事件会再次尝试 refine 标题）
    store.revealConversation(projectId, currentConv.id);
  }

  expanded.value[projectId] = true;
  store.activeConversationId = null;
}

// ── 回收站操作 ──────────────────────────────────────────

/** 恢复回收站中的会话到原项目。 */
function handleRestore(convId: string, projectId: string) {
  const project = store.projects.find((p) => p.id === projectId);
  const projectName = project?.name ?? "未知项目";
  const ok = store.restoreConversation(convId, projectId);
  if (ok) {
    expanded.value[projectId] = true;
    message.success(`已恢复会话到: ${projectName}`);
  }
}

/** 彻底删除回收站中的会话（物理删除，不可恢复）。 */
function handlePermanentDelete(convId: string) {
  const item = store.trashConversations.find((c) => c.id === convId);
  const title = item?.title ?? "未知会话";
  store.permanentDeleteConversation(convId);
  message.success(`已彻底删除: ${title}`);
}

/** 清空回收站：物理删除所有已标记删除的会话数据。 */
async function handleClearTrash() {
  const count = await store.purgeExpiredTrash(0); // 0 天 = 全部清空
  message.success(`已清空回收站，共删除 ${count} 条会话`);
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
            <!-- 项目名：右键菜单中重命名 -->
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
              @contextmenu="onConvContextMenu($event, project.id, conv.id)"
            >
              <!-- 会话名：双击进入内联重命名，Enter/blur 确认，Escape 取消 -->
              <span
                v-if="!renamingConvInfo || renamingConvInfo.convId !== conv.id"
                class="conv-title"
                @dblclick.stop="startRenameConversation(project.id, conv.id)"
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

    <!-- 回收站：固定在项目列表下方、设置按钮上方 -->
    <div class="sidebar-divider sidebar-divider--trash" />
    <div class="trash-section" :class="{ dark: themeStore.isDark }">
      <!-- 回收站标题行：点击展开/折叠 -->
      <div
        class="trash-header"
        :class="{ expanded: store.trashExpanded }"
        @click="store.trashExpanded = !store.trashExpanded"
      >
        <!-- 折叠箭头 -->
        <svg
          class="trash-chevron"
          :class="{ expanded: store.trashExpanded }"
          width="12"
          height="12"
          viewBox="0 0 12 12"
          fill="none"
        >
          <path d="M4 2l4 4-4 4" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        <!-- 垃圾桶图标 -->
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" class="trash-icon">
          <path d="M2.5 4.5l.8 7.5a1 1 0 001 .98h5.4a1 1 0 001-.98l.8-7.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
          <path d="M1.5 4.5h11M9.5 4.5V3a1 1 0 00-1-1h-3a1 1 0 00-1 1v1.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
        </svg>
        <span class="trash-label">回收站</span>
        <span v-if="store.trashConversations.length > 0" class="trash-count">{{ store.trashConversations.length }}</span>
        <!-- 清空回收站：仅展开且非空时显示 -->
        <NButton
          v-if="store.trashExpanded && store.trashConversations.length > 0"
          size="tiny"
          quaternary
          class="trash-clear-btn"
          @click.stop="handleClearTrash"
          title="彻底清空回收站"
        >
          <template #icon>
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
              <path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
            </svg>
          </template>
        </NButton>
      </div>

      <!-- 回收站展开列表 -->
      <div v-if="store.trashExpanded" class="trash-list">
        <div
          v-for="item in store.trashConversations"
          :key="item.id"
          class="trash-item"
        >
          <span class="trash-item-title">{{ item.title || "（无标题）" }}</span>
          <span class="trash-item-time">{{ formatRelativeTime(item.deletedAt ?? item.updatedAt) }}</span>
          <!-- hover 显示操作按钮 -->
          <div class="trash-item-actions">
            <NTooltip trigger="hover">
              <template #trigger>
                <NButton size="tiny" quaternary class="trash-action-btn" @click.stop="handleRestore(item.id, item.projectId ?? '')">
                  <template #icon>
                    <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                      <path d="M2 6a4 4 0 016.5-2.5M10 6a4 4 0 01-6.5 2.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
                      <path d="M8.5 1v2.5H6M3.5 11V8.5H6" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
                    </svg>
                  </template>
                </NButton>
              </template>
              恢复会话
            </NTooltip>
            <NTooltip trigger="hover">
              <template #trigger>
                <NButton size="tiny" quaternary class="trash-action-btn trash-action-btn--danger" @click.stop="handlePermanentDelete(item.id)">
                  <template #icon>
                    <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                      <path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
                    </svg>
                  </template>
                </NButton>
              </template>
              彻底删除（不可恢复）
            </NTooltip>
          </div>
        </div>
        <!-- 空回收站提示 -->
        <div v-if="store.trashConversations.length === 0" class="trash-empty">
          回收站为空
        </div>
      </div>
    </div>

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
            <!-- 重命名图标：铅笔 -->
            <svg v-if="opt.key === 'rename'" width="14" height="14" viewBox="0 0 14 14" fill="none" class="context-menu-icon">
              <path d="M10.5 1.5l2 2-9 9H1.5v-2l9-9z" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M9 3l2 2" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
            </svg>
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

    <!-- 项目重命名弹窗：居中显示，匹配当前桌面端视觉风格 -->
    <NModal
      :show="showRenameDialog"
      @update:show="(v) => { if (!v) closeRenameDialog(); }"
      :mask-closable="true"
      transform-origin="center"
    >
      <div class="rename-dialog" :class="{ dark: themeStore.isDark }">
        <div class="rename-dialog-header">
          <span class="rename-dialog-title">重命名项目</span>
        </div>
        <div class="rename-dialog-body">
          <NInput
            v-model:value="renameDialogName"
            size="large"
            placeholder="输入项目名称"
            :autofocus="true"
            @keydown.enter="confirmRenameDialog"
            @keydown.escape="closeRenameDialog"
          />
        </div>
        <div class="rename-dialog-footer">
          <NButton quaternary size="medium" @click="closeRenameDialog">取消</NButton>
          <NButton type="primary" size="medium" @click="confirmRenameDialog">确认</NButton>
        </div>
      </div>
    </NModal>
  </div>
</template>

<style scoped>
/* ═══════════════════════════════════════════
   侧边栏双模式设计
   浅色 — HelloKitty 暖粉底衬 (#fef0f3)
   深色 — 暖灰基底 + 粉色系环境微光 + 毛玻璃悬浮
   ═══════════════════════════════════════════ */
.sidebar {
  width: 260px;
  min-width: 260px;
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: #fef0f3;
  border-right: 1px solid rgba(0, 0, 0, 0.06);
  user-select: none;
  position: relative;
  overflow: hidden;
  transition: background-color 0.3s ease, border-color 0.3s ease;
}

/* 深色模式：暖灰基底带微量玫瑰底色，靠色温本身营造氛围，不加光斑 */
.sidebar.dark {
  background: #1f1c1d;
  border-right: 1px solid rgba(255, 255, 255, 0.06);
}

/* 顶部区块（新会话 + 项目标题）：毛玻璃悬浮 */
.new-conv-section,
.project-header {
  position: relative;
  z-index: 1;
}

.sidebar.dark .new-conv-section {
  background: rgba(255, 240, 245, 0.03);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  border-bottom: 1px solid rgba(255, 220, 230, 0.05);
}

.sidebar.dark .project-header {
  background: rgba(255, 240, 245, 0.025);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
}

/* 底部设置区：毛玻璃悬浮 */
.sidebar-bottom {
  position: relative;
  z-index: 1;
  padding: 6px 10px;
}

.sidebar.dark .sidebar-bottom {
  background: rgba(255, 240, 245, 0.03);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  border-top: 1px solid rgba(255, 220, 230, 0.05);
}

/* 项目列表滚动区：透出带光晕的基底 */
.sidebar.dark .project-list-scroll {
  position: relative;
  z-index: 0;
}

/* Shared divider */
.sidebar-divider {
  height: 1px;
  background: rgba(0, 0, 0, 0.06);
  flex-shrink: 0;
  margin: 0 12px;
}

.sidebar.dark .sidebar-divider {
  background: rgba(255, 255, 255, 0.04);
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
  align-items: baseline;        /* 不同字号（13px/11px）共享同一基线，视觉更整齐 */
  padding: 3px 12px 5px 48px;   /* 左内边距 48px = 会话文字与上方项目名精确对齐，上下 3px 紧凑间距 */
  gap: 8px;
  font-size: 13px;
  line-height: 1.4;
  cursor: pointer;
  transition: background-color 0.12s ease;
  border-radius: 4px;
  margin: 0 6px;
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

/* Conversation title — 轻量风格，统一字重 400，过长省略号截断 */
.conv-title {
  flex: 1;
  min-width: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-size: 13px;
  font-weight: 400;
  line-height: inherit;
  color: #374151;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

.sidebar.dark .conv-title {
  color: #e4e4e7;
}

.conversation-item.active .conv-title {
  color: #111827;
}

.sidebar.dark .conversation-item.active .conv-title {
  color: #ffffff;
}

/* Relative timestamp */
.conv-time {
  flex-shrink: 0;
  font-size: 11px;
  line-height: inherit;
  color: #9ca3af;
  white-space: nowrap;
  margin-left: auto;           /* 靠右对齐，与会话文字同行最右侧 */
}

.sidebar.dark .conv-time {
  color: #6b7280;
}

.conversation-item.active .conv-time {
  color: #9ca3af;
}

.sidebar.dark .conversation-item.active .conv-time {
  color: #9ca3af;
}

/* Empty state */
.conv-empty {
  padding: 6px 12px 6px 48px;
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

/* ── 项目重命名居中弹窗：匹配桌面端视觉风格，亮暗双模式 ── */

/* NModal 遮罩层微调 */
:deep(.n-modal-mask) {
  backdrop-filter: blur(2px);
}

/* 弹窗卡片：胶囊圆角 + 微凸起阴影，亮暗模式下层次分开 */
.rename-dialog {
  width: 380px;
  max-width: calc(100vw - 48px);
  background: #ffffff;
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 16px;
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.1),
    0 2px 8px rgba(0, 0, 0, 0.04);
  padding: 0;
  overflow: hidden;
}

.rename-dialog.dark {
  background: #25252b;
  border-color: rgba(255, 255, 255, 0.06);
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.5),
    0 2px 8px rgba(0, 0, 0, 0.3);
}

/* 弹窗头部：标题行 */
.rename-dialog-header {
  padding: 16px 20px 0;
}

.rename-dialog-title {
  font-size: 15px;
  font-weight: 600;
  color: #1a1a1a;
  letter-spacing: -0.01em;
}

.rename-dialog.dark .rename-dialog-title {
  color: #e8e8ed;
}

/* 弹窗主体：输入框区域 */
.rename-dialog-body {
  padding: 16px 20px;
}

.rename-dialog-body :deep(.n-input) {
  --n-border: 1px solid rgba(0, 0, 0, 0.1);
  --n-border-radius: 10px;
}

.rename-dialog.dark .rename-dialog-body :deep(.n-input) {
  --n-border: 1px solid rgba(255, 255, 255, 0.1);
}

/* 弹窗底部：按钮组右对齐 */
.rename-dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 0 20px 16px;
}

/* Naive UI NModal 内容容器居中偏移修正 */
:deep(.n-modal) {
  display: flex;
  align-items: center;
  justify-content: center;
}

/* ── 回收站 ──────────────────────────────────────────── */

.sidebar-divider--trash {
  margin: 8px 12px 4px;
  background: rgba(0, 0, 0, 0.06);
}

.sidebar.dark .sidebar-divider--trash {
  background: rgba(255, 255, 255, 0.04);
}

.trash-section {
  padding: 0 6px;
  flex-shrink: 0;
}

/* 回收站标题行 */
.trash-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  border-radius: 5px;
  cursor: pointer;
  transition: background-color 0.12s ease;
  user-select: none;
}

.trash-header:hover {
  background: rgba(0, 0, 0, 0.04);
}

.trash-section.dark .trash-header:hover {
  background: rgba(255, 255, 255, 0.04);
}

/* 折叠箭头 */
.trash-chevron {
  flex-shrink: 0;
  color: #9ca3af;
  transition: transform 0.15s ease;
}

.trash-chevron.expanded {
  transform: rotate(90deg);
}

.trash-section.dark .trash-chevron {
  color: #6b7280;
}

/* 垃圾桶图标 */
.trash-icon {
  flex-shrink: 0;
  color: #9ca3af;
}

.trash-section.dark .trash-icon {
  color: #6b7280;
}

/* 回收站文字 */
.trash-label {
  flex: 1;
  font-size: 13px;
  font-weight: 500;
  color: #6b7280;
}

.trash-section.dark .trash-label {
  color: #9ca3af;
}

/* 回收站计数 */
.trash-count {
  font-size: 11px;
  font-weight: 600;
  color: #9ca3af;
  background: rgba(0, 0, 0, 0.06);
  border-radius: 8px;
  padding: 1px 6px;
  min-width: 18px;
  text-align: center;
}

.trash-section.dark .trash-count {
  color: #6b7280;
  background: rgba(255, 255, 255, 0.08);
}

/* 清空按钮 */
.trash-clear-btn {
  opacity: 0.6;
  color: #9ca3af;
  flex-shrink: 0;
}

.trash-clear-btn:hover {
  opacity: 1;
  color: #ef4444;
}

/* 回收站列表 */
.trash-list {
  padding: 2px 0 4px;
}

/* 回收站条目 */
.trash-item {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px 4px 28px;
  border-radius: 4px;
  cursor: default;
  transition: background-color 0.12s ease;
}

.trash-item:hover {
  background: rgba(0, 0, 0, 0.03);
}

.trash-section.dark .trash-item:hover {
  background: rgba(255, 255, 255, 0.03);
}

/* 回收站条目标题 — 灰色/删除线表示已删除状态 */
.trash-item-title {
  flex: 1;
  min-width: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-size: 12px;
  color: #9ca3af;
  text-decoration: line-through;
}

.trash-section.dark .trash-item-title {
  color: #6b7280;
}

/* 回收站条目时间 */
.trash-item-time {
  flex-shrink: 0;
  font-size: 10px;
  color: #c4c4c4;
  white-space: nowrap;
}

.trash-section.dark .trash-item-time {
  color: #52525b;
}

/* 操作按钮容器：默认隐藏，hover 时显示 */
.trash-item-actions {
  display: none;
  gap: 2px;
  flex-shrink: 0;
  margin-left: 4px;
}

.trash-item:hover .trash-item-actions {
  display: flex;
}

/* 操作按钮 */
.trash-action-btn {
  color: #9ca3af;
  opacity: 0.6;
}

.trash-action-btn:hover {
  opacity: 1;
  color: #374151;
}

.trash-section.dark .trash-action-btn {
  color: #6b7280;
}

.trash-section.dark .trash-action-btn:hover {
  color: #e4e4e7;
}

/* 彻底删除按钮 hover 变红 */
.trash-action-btn--danger:hover {
  color: #ef4444 !important;
}

.trash-section.dark .trash-action-btn--danger:hover {
  color: #f87171 !important;
}

/* 回收站空状态 */
.trash-empty {
  padding: 4px 8px 4px 28px;
  font-size: 11px;
  color: #c4c4c4;
  font-style: italic;
}

.trash-section.dark .trash-empty {
  color: #52525b;
}

/* NDropdown / NSelect 下拉菜单视觉样式统一由 App.vue 全局样式控制（Naive UI Teleport 到 body，scoped 穿透无效） */
</style>
