<script setup lang="ts">
import { ref } from "vue";
import { Icon } from "@iconify/vue";
import type { DiffFile } from "../../utils/diffParser";
import ReviewDiffItem from "./ReviewDiffItem.vue";

// 变更文件列表，每个文件可展开查看 diff 详情

defineProps<{
  files: DiffFile[];
}>();

const expandedFiles = ref<Set<string>>(new Set());
const collapsedAll = ref(false);

function toggleExpand(path: string) {
  const next = new Set(expandedFiles.value);
  if (next.has(path)) next.delete(path);
  else next.add(path);
  expandedFiles.value = next;
}

function isExpanded(path: string): boolean {
  return expandedFiles.value.has(path);
}

/** 全部折叠/展开 */
function toggleAll() {
  if (collapsedAll.value) {
    expandedFiles.value = new Set();
    collapsedAll.value = true;
  }
  collapsedAll.value = !collapsedAll.value;
  if (collapsedAll.value) {
    expandedFiles.value = new Set();
  }
}

/** 文件扩展名 -> Iconify logos 图标名映射，使用各语言/工具官方 Logo */
const EXT_ICON_MAP: Record<string, string> = {
  ts:   "logos:typescript-icon",
  tsx:  "logos:typescript-icon",
  js:   "logos:javascript",
  jsx:  "logos:javascript",
  mjs:  "logos:javascript",
  cjs:  "logos:javascript",
  vue:  "logos:vue",
  html: "logos:html-5",
  htm:  "logos:html-5",
  css:  "logos:css-3",
  scss: "logos:sass",
  less: "logos:less",
  rs:   "logos:rust",
  go:   "logos:go",
  py:   "logos:python",
  pyi:  "logos:python",
  pyx:  "logos:python",
  java: "logos:java",
  kt:   "logos:kotlin",
  kts:  "logos:kotlin",
  cs:   "logos:c-sharp",
  php:  "logos:php",
  rb:   "logos:ruby",
  swift:"logos:swift",
  c:    "logos:c",
  h:    "logos:c",
  cpp:  "logos:c-plusplus",
  cxx:  "logos:c-plusplus",
  cc:   "logos:c-plusplus",
  hpp:  "logos:c-plusplus",
  zig:  "logos:zig",
  json: "logos:json",
  yaml: "logos:yaml",
  yml:  "logos:yaml",
  toml: "logos:toml",
  xml:  "logos:xml",
  svg:  "logos:svg",
  sql:  "logos:postgresql",
  graphql:"logos:graphql",
  gql:  "logos:graphql",
  proto:"logos:protobuf",
  sh:   "logos:bash-icon",
  bash: "logos:bash-icon",
  zsh:  "logos:bash-icon",
  md:   "logos:markdown",
  mdx:  "logos:markdown",
  dockerfile: "logos:docker-icon",
  svelte: "logos:svelte-icon",
  ex:   "logos:elixir",
  exs:  "logos:elixir",
  scala:"logos:scala",
  clj:  "logos:clojure",
  cljs: "logos:clojure",
  hs:   "logos:haskell",
  erl:  "logos:erlang",
};

/** 根据文件路径返回 Iconify 图标名，未匹配返回空由模板兜底显示文本标签 */
function langIcon(filePath: string): string | null {
  const ext = filePath.split(".").pop()?.toLowerCase() || "";
  // 特殊处理：无扩展名的文件（如 Dockerfile）
  const base = filePath.split("/").pop()?.toLowerCase() || "";
  if (EXT_ICON_MAP[base]) return EXT_ICON_MAP[base];
  if (EXT_ICON_MAP[ext]) return EXT_ICON_MAP[ext];
  return null;
}

/** 未匹配图标时，取扩展名前 3 个字符大写作为兜底文本标签 */
function langFallbackLabel(filePath: string): string {
  const ext = filePath.split(".").pop()?.toLowerCase() || "";
  return ext.slice(0, 3).toUpperCase() || "?";
}

</script>

<template>
  <div class="file-list">
    <!-- 工具栏：全部折叠/展开 + 统计 -->
    <div class="file-list-toolbar">
      <button class="file-list-toggle-all" @click="toggleAll">
        {{ collapsedAll ? "展开全部" : "折叠全部" }}
      </button>
      <span class="file-list-summary">{{ files.length }} 个文件</span>
    </div>

    <!-- 文件列表 -->
    <div
      v-for="file in files"
      :key="file.path"
      class="file-item"
    >
      <!-- 文件头部：语言图标 + 状态圆点 + 路径 + 变更统计 + 展开按钮 -->
      <div
        class="file-item-header"
        :class="{ expanded: isExpanded(file.path) }"
        @click="toggleExpand(file.path)"
      >
        <!-- 语言官方 Logo：通过 Iconify logos 图标集展示真实产品商标 -->
        <Icon
          v-if="langIcon(file.path)"
          :icon="langIcon(file.path) ?? ''"
          class="file-lang-icon"
          width="13"
          height="14"
        />
        <!-- 未匹配图标时兜底显示文本标签 -->
        <span
          v-else
          class="file-lang-fallback"
        >{{ langFallbackLabel(file.path) }}</span>
        <span class="file-path">{{ file.path }}</span>
        <span class="file-change-count">
          <span class="add-count">+{{ file.additions }}</span>
          <span class="del-count">-{{ file.deletions }}</span>
        </span>
        <svg
          class="file-expand-chevron"
          :class="{ rotated: isExpanded(file.path) }"
          width="12" height="12" viewBox="0 0 12 12" fill="none"
        >
          <path d="M4 2.5l3.5 3.5L4 9.5" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </div>

      <!-- 展开的 diff 内容 -->
      <div v-if="isExpanded(file.path)" class="file-diff-expanded">
        <ReviewDiffItem :file="file" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.file-list {
  padding: 8px 0;
}

/* ── 工具栏 ── */
.file-list-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 14px 8px;
}

.file-list-toggle-all {
  font-size: 11px;
  font-weight: 500;
  color: #6366f1;
  background: none;
  border: none;
  cursor: pointer;
  padding: 2px 0;
}

.file-list-toggle-all:hover {
  color: #4f46e5;
}

.dark .file-list-toggle-all {
  color: #818cf8;
}

.dark .file-list-toggle-all:hover {
  color: #a5b4fc;
}

.file-list-summary {
  font-size: 11px;
  color: #9ca3af;
}

.dark .file-list-summary {
  color: #6b7280;
}

/* ── 文件条目 ── */
.file-item {
  border-bottom: 1px solid rgba(0, 0, 0, 0.03);
}

.dark .file-item {
  border-bottom-color: rgba(255, 255, 255, 0.03);
}

.file-item:last-child {
  border-bottom: none;
}

/* ── 文件头部 ── */
.file-item-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  cursor: pointer;
  transition: background 0.12s ease;
}

.file-item-header:hover {
  background: rgba(0, 0, 0, 0.02);
}

.file-item-header.expanded {
  background: rgba(99, 102, 241, 0.03);
}

.dark .file-item-header:hover {
  background: rgba(255, 255, 255, 0.02);
}

.dark .file-item-header.expanded {
  background: rgba(129, 140, 248, 0.04);
}

/* 语言官方 Logo 图标：Iconify 渲染的 SVG，保持清晰锐利 */
.file-lang-icon {
  flex-shrink: 0;
  border-radius: 2px;
}

/* 未匹配图标时的兜底文本标签 */
.file-lang-fallback {
  min-width: 22px;
  height: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 3px;
  font-size: 9px;
  font-weight: 600;
  font-family: "Inter", "SF Pro Display", -apple-system, BlinkMacSystemFont, sans-serif;
  letter-spacing: 0.02em;
  flex-shrink: 0;
  padding: 0 3px;
  color: #9ca3af;
  background: rgba(156, 163, 175, 0.1);
}

/* 文件路径 */
.file-path {
  flex: 1;
  font-size: 12.5px;
  font-weight: 460;
  color: #374151;
  font-family: "SF Mono", "Cascadia Code", "Fira Code", monospace;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}

.dark .file-path {
  color: #d1d5db;
}

/* 变更计数 */
.file-change-count {
  display: flex;
  gap: 4px;
  font-size: 11px;
  font-family: "SF Mono", "Cascadia Code", monospace;
  flex-shrink: 0;
}

.add-count { color: #16a34a; }
.del-count { color: #dc2626; }

.dark .add-count { color: #4ade80; }
.dark .del-count { color: #f87171; }

/* 展开箭头 */
.file-expand-chevron {
  color: #9ca3af;
  flex-shrink: 0;
  transition: transform 0.2s ease;
}

.file-expand-chevron.rotated {
  transform: rotate(90deg);
}

.dark .file-expand-chevron {
  color: #6b7280;
}

/* ── 展开的 diff 区域 ── */
.file-diff-expanded {
  border-top: 1px solid rgba(0, 0, 0, 0.04);
  animation: diffExpandIn 0.18s ease;
}

.dark .file-diff-expanded {
  border-top-color: rgba(255, 255, 255, 0.04);
}

@keyframes diffExpandIn {
  from {
    opacity: 0;
    max-height: 0;
  }
  to {
    opacity: 1;
    max-height: 2000px;
  }
}
</style>
