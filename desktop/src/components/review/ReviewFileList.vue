<script setup lang="ts">
import { ref } from "vue";
import { Icon } from "@iconify/vue";
import { useReviewStore } from "../../stores/review";
import type { DiffFile } from "../../utils/diffParser";
import ReviewDiffItem from "./ReviewDiffItem.vue";

// 变更文件列表：默认所有文件折叠（仅显示摘要），通过工具栏可切换折叠/展开

defineProps<{
  files: DiffFile[];
}>();

const store = useReviewStore();
const expandedFiles = ref<Set<string>>(new Set());

function toggleExpand(path: string) {
  const next = new Set(expandedFiles.value);
  if (next.has(path)) next.delete(path);
  else next.add(path);
  expandedFiles.value = next;
}

function isExpanded(path: string): boolean {
  // 全局折叠模式：所有文件折叠
  if (store.collapseAll) return false;
  return expandedFiles.value.has(path);
}

/** 生成稳定的文件锚点 ID（与工具栏保持一致） */
function fileId(path: string): string {
  return "review-file-" + path.replace(/[^a-zA-Z0-9_-]/g, "_");
}

// ── 语言图标映射（与之前一致） ──
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

function langIcon(filePath: string): string | null {
  const ext = filePath.split(".").pop()?.toLowerCase() || "";
  const base = filePath.split("/").pop()?.toLowerCase() || "";
  if (EXT_ICON_MAP[base]) return EXT_ICON_MAP[base];
  if (EXT_ICON_MAP[ext]) return EXT_ICON_MAP[ext];
  return null;
}

function langFallbackLabel(filePath: string): string {
  const ext = filePath.split(".").pop()?.toLowerCase() || "";
  return ext.slice(0, 3).toUpperCase() || "?";
}
</script>

<template>
  <div class="file-list">
    <!-- 文件列表 -->
    <div
      v-for="file in files"
      :key="file.path"
      :id="fileId(file.path)"
      class="file-item"
    >
      <!-- 文件头部：语言图标 + 路径 + 变更统计 + 展开/折叠箭头 -->
      <div
        class="file-item-header"
        :class="{ expanded: isExpanded(file.path) }"
        @click="toggleExpand(file.path)"
      >
        <Icon
          v-if="langIcon(file.path)"
          :icon="langIcon(file.path) ?? ''"
          class="file-lang-icon"
          width="13"
          height="14"
        />
        <span v-else class="file-lang-fallback">{{ langFallbackLabel(file.path) }}</span>
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
  padding: 4px 0;
}

/* ── 文件条目 ── */
.file-item {
  border-bottom: 1px solid rgba(0, 0, 0, 0.03);
  scroll-margin-top: 8px;
}

.dark .file-item {
  border-bottom-color: rgba(255, 255, 255, 0.03);
}

.file-item:last-child {
  border-bottom: none;
}

/* ── 导航定位高亮闪烁动画 ── */
.file-item:global(.file-flash) {
  animation: fileFlash 1.2s ease;
}

@keyframes fileFlash {
  0%   { background: rgba(99, 102, 241, 0.12); }
  60%  { background: rgba(99, 102, 241, 0.04); }
  100% { background: transparent; }
}

/* ── 文件头部 ── */
.file-item-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 9px 14px;
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

/* 语言 Logo */
.file-lang-icon {
  flex-shrink: 0;
  border-radius: 2px;
}

/* 兜底文本标签 */
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
  font-size: 12px;
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
  font-size: 11.5px;
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

/* ── 展开区域 ── */
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
