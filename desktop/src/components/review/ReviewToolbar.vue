<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from "vue";
import { useReviewStore } from "../../stores/review";
import { useThemeStore } from "../../stores/theme";
import type { DiffScope } from "../../utils/diffParser";

// 审查工具栏：范围选择器 + 显示目录 + 定位至文件 + 折叠全部 + 刷新

const store = useReviewStore();
const themeStore = useThemeStore();

const scopeOptions: { value: DiffScope; label: string }[] = [
  { value: "uncommitted", label: "未提交变更" },
  { value: "branch", label: "分支变更" },
  { value: "last-turn", label: "最近一轮" },
];

// ── 范围下拉 ──
const scopeDropdownOpen = ref(false);
const scopeTriggerRef = ref<HTMLElement | null>(null);

const currentLabel = computed(() => {
  const opt = scopeOptions.find((o) => o.value === store.scope);
  return opt?.label ?? "未提交变更";
});

const dropdownOptions = computed(() =>
  scopeOptions.filter((o) => o.value !== store.scope)
);

async function changeScope(s: DiffScope) {
  if (s === store.scope && !store.error) return;
  scopeDropdownOpen.value = false;
  await store.fetchDiff(s);
  if (!store.error) {
    store.scope = s;
  }
}

function toggleScopeDropdown() {
  scopeDropdownOpen.value = !scopeDropdownOpen.value;
}

// ── 文件导航下拉 ──
const fileDropdownOpen = ref(false);
const fileTriggerRef = ref<HTMLElement | null>(null);

/** 下拉文件列表：按路径排序 */
const fileOptions = computed(() =>
  [...store.diffFiles]
    .sort((a, b) => a.path.localeCompare(b.path))
    .map((f) => ({
      label: f.path,
      key: f.path,
      additions: f.additions,
      deletions: f.deletions,
    }))
);

function navigateToFile(filePath: string) {
  fileDropdownOpen.value = false;
  store.setActiveFile(filePath);
  // 延迟滚动确保 DOM 已更新
  requestAnimationFrame(() => {
    const el = document.getElementById(fileId(filePath));
    if (el) {
      el.scrollIntoView({ behavior: "smooth", block: "center" });
      el.classList.add("file-flash");
      setTimeout(() => el.classList.remove("file-flash"), 1200);
    }
  });
}

/** 生成稳定的文件锚点 ID */
function fileId(path: string): string {
  return "review-file-" + path.replace(/[^a-zA-Z0-9_-]/g, "_");
}

function toggleFileDropdown() {
  fileDropdownOpen.value = !fileDropdownOpen.value;
}

// ── 显示项目目录：在审查面板内展示目录树 ──
function showDirectory() {
  store.toggleDirTree();
}

// ── 全局点击关闭下拉 ──
function onGlobalClick(e: MouseEvent) {
  if (scopeDropdownOpen.value && scopeTriggerRef.value && !scopeTriggerRef.value.contains(e.target as Node)) {
    scopeDropdownOpen.value = false;
  }
  if (fileDropdownOpen.value && fileTriggerRef.value && !fileTriggerRef.value.contains(e.target as Node)) {
    fileDropdownOpen.value = false;
  }
}

onMounted(() => document.addEventListener("click", onGlobalClick));
onUnmounted(() => document.removeEventListener("click", onGlobalClick));
</script>

<template>
  <div class="review-toolbar" :class="{ dark: themeStore.isDark }">
    <!-- 左侧按钮组 -->
    <div class="toolbar-left">
      <!-- 范围下拉 -->
      <div ref="scopeTriggerRef" class="selector-wrap">
        <button
          class="tb-btn tb-btn--scope"
          :class="{ open: scopeDropdownOpen }"
          @click.stop="toggleScopeDropdown"
        >
          <svg class="tb-icon" width="13" height="13" viewBox="0 0 13 13" fill="none">
            <rect x="1.5" y="1.5" width="10" height="10" rx="2" stroke="currentColor" stroke-width="1.2"/>
            <path d="M4.5 1.5v10M1.5 5.5h10" stroke="currentColor" stroke-width="1" opacity="0.5"/>
          </svg>
          <span class="tb-label">{{ currentLabel }}</span>
          <svg class="tb-chevron" :class="{ flipped: scopeDropdownOpen }" width="10" height="10" viewBox="0 0 10 10" fill="none">
            <path d="M2.5 3.5L5 6l2.5-2.5" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>
        <Transition name="drop">
          <div v-if="scopeDropdownOpen" class="dropdown-menu" :class="{ dark: themeStore.isDark }">
            <button v-for="opt in dropdownOptions" :key="opt.value" class="dropdown-item" @click.stop="changeScope(opt.value)">
              <span>{{ opt.label }}</span>
            </button>
          </div>
        </Transition>
      </div>

      <!-- 分隔符 -->
      <div class="tb-sep"></div>

      <!-- 显示目录：在审查面板内展示项目目录树 -->
      <button
        class="tb-btn tb-btn--icon"
        :class="{ active: store.showDirTree }"
        @click="showDirectory"
        title="显示项目目录"
      >
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <path d="M2 3.5A1.5 1.5 0 013.5 2h2.1a1.5 1.5 0 011.06.44l.68.68a1.5 1.5 0 001.06.44H11.5A1.5 1.5 0 0113 5.06V11.5a1.5 1.5 0 01-1.5 1.5h-9A1.5 1.5 0 012 11.5V3.5z" stroke="currentColor" stroke-width="1.15" stroke-linejoin="round"/>
        </svg>
      </button>

      <!-- 定位至文件 -->
      <div ref="fileTriggerRef" class="selector-wrap">
        <button
          class="tb-btn tb-btn--icon"
          :class="{ open: fileDropdownOpen, disabled: store.diffFiles.length === 0 }"
          :disabled="store.diffFiles.length === 0"
          @click.stop="toggleFileDropdown"
          title="定位至文件"
        >
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <path d="M2 4h10M2 7h8M2 10h6" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
          </svg>
        </button>
        <Transition name="drop">
          <div v-if="fileDropdownOpen" class="dropdown-menu dropdown-menu--files" :class="{ dark: themeStore.isDark }">
            <div v-if="fileOptions.length === 0" class="dropdown-empty">暂无变更文件</div>
            <button
              v-for="f in fileOptions"
              :key="f.key"
              class="dropdown-item dropdown-item--file"
              @click.stop="navigateToFile(f.key)"
            >
              <span class="file-item-path">{{ f.label }}</span>
              <span class="file-item-stats">
                <span class="file-stat-add">+{{ f.additions }}</span>
                <span class="file-stat-del">-{{ f.deletions }}</span>
              </span>
            </button>
          </div>
        </Transition>
      </div>

      <!-- 折叠全部差异 -->
      <button
        class="tb-btn tb-btn--icon"
        :class="{ active: store.collapseAll, disabled: store.diffFiles.length === 0 }"
        :disabled="store.diffFiles.length === 0"
        @click="store.toggleCollapseAll()"
        :title="store.collapseAll ? '展开全部差异' : '折叠全部差异'"
      >
        <svg v-if="!store.collapseAll" width="14" height="14" viewBox="0 0 14 14" fill="none">
          <path d="M4 5l3 3 3-3" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M2 10h10" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
        </svg>
        <svg v-else width="14" height="14" viewBox="0 0 14 14" fill="none">
          <path d="M4 9l3-3 3 3" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M2 4h10" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
        </svg>
      </button>
    </div>

    <!-- 刷新 -->
    <button class="tb-btn tb-btn--icon" @click="store.fetchDiff()" title="刷新变更">
      <svg width="13" height="13" viewBox="0 0 13 13" fill="none">
        <path d="M2.5 6.5a4 4 0 018 0" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
        <path d="M10.5 6.5a4 4 0 01-8 0" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
        <path d="M10.5 3v3.5H7" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
        <path d="M2.5 10V6.5H6" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </button>
  </div>
</template>

<style scoped>
.review-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 8px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.04);
  flex-shrink: 0;
  gap: 2px;
}

.dark .review-toolbar {
  border-bottom-color: rgba(255, 255, 255, 0.04);
}

.toolbar-left {
  display: flex;
  align-items: center;
  gap: 1px;
}

/* ── 下拉容器 ── */
.selector-wrap {
  position: relative;
}

/* ── 通用按钮基类 ── */
.tb-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  height: 28px;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: #6b7280;
  cursor: pointer;
  transition: all 0.15s ease;
  outline: none;
  white-space: nowrap;
  flex-shrink: 0;
}

.tb-btn:hover {
  background: rgba(0, 0, 0, 0.04);
  color: #374151;
}

.dark .tb-btn {
  color: #9ca3af;
}

.dark .tb-btn:hover {
  background: rgba(255, 255, 255, 0.04);
  color: #d1d5db;
}

/* 范围触发器 */
.tb-btn--scope {
  gap: 4px;
  padding: 0 8px;
  border-color: rgba(0, 0, 0, 0.06);
  background: rgba(0, 0, 0, 0.015);
}

.tb-btn--scope:hover {
  border-color: rgba(0, 0, 0, 0.1);
  background: rgba(0, 0, 0, 0.04);
}

.tb-btn--scope.open {
  border-color: rgba(99, 102, 241, 0.25);
  background: rgba(99, 102, 241, 0.06);
  color: #6366f1;
}

.dark .tb-btn--scope {
  border-color: rgba(255, 255, 255, 0.06);
  background: rgba(255, 255, 255, 0.02);
}

.dark .tb-btn--scope:hover {
  border-color: rgba(255, 255, 255, 0.1);
  background: rgba(255, 255, 255, 0.04);
}

.dark .tb-btn--scope.open {
  border-color: rgba(129, 140, 248, 0.3);
  background: rgba(129, 140, 248, 0.08);
  color: #818cf8;
}

/* 图标按钮 */
.tb-btn--icon {
  width: 28px;
  padding: 0;
}

.tb-btn--icon.open {
  background: rgba(99, 102, 241, 0.06);
  color: #6366f1;
}

.tb-btn--icon.active {
  background: rgba(99, 102, 241, 0.08);
  color: #6366f1;
}

.tb-btn--icon.disabled {
  opacity: 0.3;
  pointer-events: none;
}

.dark .tb-btn--icon.open {
  background: rgba(129, 140, 248, 0.08);
  color: #818cf8;
}

.dark .tb-btn--icon.active {
  background: rgba(129, 140, 248, 0.1);
  color: #818cf8;
}

/* 图标 */
.tb-icon {
  flex-shrink: 0;
  opacity: 0.55;
}

.tb-btn:hover .tb-icon,
.tb-btn.open .tb-icon {
  opacity: 0.85;
}

.tb-label {
  font-size: 11.5px;
  font-weight: 500;
  line-height: 1;
}

/* 箭头 */
.tb-chevron {
  flex-shrink: 0;
  opacity: 0.4;
  transition: transform 0.2s ease;
}

.tb-btn:hover .tb-chevron {
  opacity: 0.65;
}

.tb-chevron.flipped {
  transform: rotate(180deg);
  opacity: 0.75;
}

/* ── 分隔符 ── */
.tb-sep {
  width: 1px;
  height: 16px;
  background: rgba(0, 0, 0, 0.08);
  margin: 0 3px;
  flex-shrink: 0;
}

.dark .tb-sep {
  background: rgba(255, 255, 255, 0.08);
}

/* ── 下拉菜单（共享） ── */
.dropdown-menu {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  min-width: 140px;
  background: #ffffff;
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.1), 0 2px 6px rgba(0, 0, 0, 0.05);
  padding: 4px;
  z-index: 100;
  overflow: hidden;
  max-height: 320px;
  overflow-y: auto;
}

.dropdown-menu.dark {
  background: #232326;
  border-color: rgba(255, 255, 255, 0.08);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4), 0 2px 6px rgba(0, 0, 0, 0.2);
}

/* 文件列表下拉稍宽 */
.dropdown-menu--files {
  min-width: 220px;
  max-width: 300px;
}

.dropdown-item {
  display: flex;
  align-items: center;
  width: 100%;
  padding: 6px 10px;
  border: none;
  border-radius: 5px;
  background: transparent;
  color: #374151;
  font-size: 12px;
  font-weight: 450;
  cursor: pointer;
  transition: all 0.1s ease;
  text-align: left;
  white-space: nowrap;
}

.dropdown-item:hover {
  background: rgba(0, 0, 0, 0.04);
  color: #111827;
}

.dark .dropdown-item {
  color: #e4e4e7;
}

.dark .dropdown-item:hover {
  background: rgba(255, 255, 255, 0.06);
  color: #ffffff;
}

/* 文件项布局 */
.dropdown-item--file {
  justify-content: space-between;
  gap: 8px;
}

.file-item-path {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: "SF Mono", "Cascadia Code", "Fira Code", monospace;
  font-size: 11px;
}

.file-item-stats {
  display: flex;
  gap: 4px;
  font-size: 11.5px;
  font-family: "SF Mono", "Cascadia Code", monospace;
  flex-shrink: 0;
}

.file-stat-add { color: #16a34a; }
.file-stat-del { color: #dc2626; }

.dark .file-stat-add { color: #4ade80; }
.dark .file-stat-del { color: #f87171; }

.dropdown-empty {
  padding: 10px;
  font-size: 11.5px;
  color: #9ca3af;
  text-align: center;
}

/* ── 下拉动画 ── */
.drop-enter-active {
  transition: all 0.15s cubic-bezier(0.32, 0.72, 0, 1);
}
.drop-leave-active {
  transition: all 0.1s ease-in;
}
.drop-enter-from {
  opacity: 0;
  transform: translateY(-4px) scale(0.97);
}
.drop-leave-to {
  opacity: 0;
  transform: translateY(-2px) scale(0.98);
}
</style>
