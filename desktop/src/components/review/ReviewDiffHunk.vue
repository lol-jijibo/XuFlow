<script setup lang="ts">
import type { DiffHunk, DiffLine } from "../../utils/diffParser";
import { highlightLine } from "../../utils/diffParser";
import ReviewComment from "./ReviewComment.vue";

// 单个 diff hunk 渲染，对每行代码进行语法高亮

const props = defineProps<{
  hunk: DiffHunk;
  language: string;
  filePath: string;
}>();

/** 对行进行语法高亮，返回 HTML */
function highlightedHtml(line: DiffLine): string {
  const code = line.content;
  if (!code) return "&nbsp;";
  return highlightLine(code, props.language);
}

/** 行背景色类 */
function lineBgClass(type: DiffLine["type"]): string {
  switch (type) {
    case "add": return "line-add";
    case "remove": return "line-remove";
    case "header": return "line-header";
    default: return "line-context";
  }
}

/** 行号显示 */
function lineNumber(line: DiffLine, side: "old" | "new"): string {
  if (side === "old") {
    return line.oldLineNo != null ? String(line.oldLineNo) : "";
  }
  return line.newLineNo != null ? String(line.newLineNo) : "";
}
</script>

<template>
  <div class="diff-hunk">
    <!-- Hunk 头部（@@ 行） -->
    <div class="hunk-header">{{ hunk.header }}</div>

    <!-- 代码行 -->
    <div
      v-for="(line, lIdx) in hunk.lines"
      :key="lIdx"
      class="diff-line"
      :class="[lineBgClass(line.type)]"
    >
      <!-- 行号 -->
      <span class="line-no line-no-old">{{ lineNumber(line, "old") }}</span>
      <span class="line-no line-no-new">{{ lineNumber(line, "new") }}</span>

      <!-- 行前缀标记 -->
      <span class="line-prefix">
        <template v-if="line.type === 'add'">+</template>
        <template v-else-if="line.type === 'remove'">-</template>
        <template v-else>&nbsp;</template>
      </span>

      <!-- 语法高亮的代码内容 -->
      <span class="line-content" v-html="highlightedHtml(line)" />

      <!-- 行级评论数量徽章 -->
      <span v-if="line.comments.length > 0" class="line-comment-count">
        {{ line.comments.length }}
      </span>
    </div>

    <!-- 该 hunk 的行级评论合集 -->
    <div v-for="line in hunk.lines" :key="'c-' + line.content" class="hunk-comments">
      <ReviewComment
        v-for="comment in line.comments"
        :key="comment.id"
        :comment="comment"
        :filePath="filePath"
        :lineContent="line.content"
      />
    </div>
  </div>
</template>

<style scoped>
.diff-hunk {
  font-family: "SF Mono", "Cascadia Code", "Fira Code", "JetBrains Mono", monospace;
  font-size: 12px;
  line-height: 1.55;
}

/* ── Hunk 头部 ── */
.hunk-header {
  padding: 4px 8px;
  font-size: 11px;
  color: #6366f1;
  background: rgba(99, 102, 241, 0.04);
  border-radius: 0;
  user-select: text;
  cursor: default;
}

.dark .hunk-header {
  color: #a5b4fc;
  background: rgba(129, 140, 248, 0.05);
}

/* ── 代码行 ── */
.diff-line {
  display: flex;
  align-items: flex-start;
  gap: 0;
  padding: 0 4px;
  min-height: 20px;
  position: relative;
  transition: background 0.1s ease;
}

.diff-line:hover {
  filter: brightness(0.96);
}

.dark .diff-line:hover {
  filter: brightness(1.1);
}

/* 行背景色 */
.line-add {
  background: rgba(34, 197, 94, 0.08);
}

.line-remove {
  background: rgba(239, 68, 68, 0.06);
}

.line-context {
  background: transparent;
}

.line-header {
  background: rgba(99, 102, 241, 0.04);
}

.dark .line-add {
  background: rgba(34, 197, 94, 0.1);
}

.dark .line-remove {
  background: rgba(239, 68, 68, 0.08);
}

.dark .line-header {
  background: rgba(129, 140, 248, 0.05);
}

/* ── 行号 ── */
.line-no {
  width: 36px;
  flex-shrink: 0;
  text-align: right;
  padding-right: 8px;
  font-size: 11px;
  color: #9ca3af;
  user-select: none;
}

.line-no:hover {
  color: #6366f1;
}

.dark .line-no {
  color: #6b7280;
}

.dark .line-no:hover {
  color: #a5b4fc;
}

/* ── 行前缀（+/-/ ） ── */
.line-prefix {
  width: 14px;
  flex-shrink: 0;
  text-align: center;
  font-weight: 600;
  font-size: 11px;
  user-select: none;
}

.line-add .line-prefix { color: #16a34a; }
.line-remove .line-prefix { color: #dc2626; }
.line-context .line-prefix { color: transparent; }

.dark .line-add .line-prefix { color: #4ade80; }
.dark .line-remove .line-prefix { color: #f87171; }

/* ── 高亮代码内容 ── */
.line-content {
  flex: 1;
  min-width: 0;
  white-space: pre-wrap;
  word-break: break-all;
  overflow-wrap: break-word;
  padding-left: 4px;
}

/* highlight.js 渲染的 span 保持 diff 背景透明 */
.line-content :deep(span) {
  background: transparent !important;
}

/* ── 行评论计数徽章 ── */
.line-comment-count {
  width: 16px;
  height: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background: #6366f1;
  color: #fff;
  font-size: 9px;
  font-weight: 600;
  flex-shrink: 0;
  margin-left: 4px;
}

.dark .line-comment-count {
  background: #818cf8;
}

/* ── 评论容器 ── */
.hunk-comments {
  display: contents;
}
</style>
