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
/* ═══════════════════════════════════════════
   Diff Hunk 代码展示区：模拟 IDE 编辑器观感
   浅色模式 — 亮白底 + 深色文字高亮
   深色模式 — 暗色底 + 浅色文字高亮
   ═══════════════════════════════════════════ */

.diff-hunk {
  font-family: "SF Mono", "Cascadia Code", "Fira Code", "JetBrains Mono", monospace;
  font-size: 12px;
  line-height: 1.6;
  /* 代码区专属底衬：浅灰白，与面板 #fafafa 形成卡片层次 */
  background: #fcfcfc;
  border-radius: 6px;
  margin: 6px 8px;
  overflow: hidden;
  border: 1px solid rgba(0, 0, 0, 0.06);
}

.dark .diff-hunk {
  /* 深灰黑底衬，模拟 IDE 暗色编辑器 */
  background: #121214;
  border-color: rgba(255, 255, 255, 0.06);
}

/* ── Hunk 头部 ── */
.hunk-header {
  padding: 5px 10px;
  font-size: 11px;
  color: #475569;
  background: rgba(0, 0, 0, 0.03);
  user-select: text;
  cursor: default;
  border-bottom: 1px solid rgba(0, 0, 0, 0.04);
}

.dark .hunk-header {
  color: #94a3b8;
  background: rgba(255, 255, 255, 0.03);
  border-bottom-color: rgba(255, 255, 255, 0.04);
}

/* ── 代码行 ── */
.diff-line {
  display: flex;
  align-items: flex-start;
  gap: 0;
  padding: 0 6px;
  min-height: 22px;
  position: relative;
}

/* hover 时用 ::after 叠加半透明遮罩代替 filter，避免 GPU 合成层导致文字模糊 */
.diff-line:hover::after {
  content: "";
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.03);
  pointer-events: none;
  z-index: 0;
}

.dark .diff-line:hover::after {
  background: rgba(255, 255, 255, 0.04);
}

/* ── 行背景色：模拟 IDE diff 视图 ── */
.line-add {
  background: rgba(34, 197, 94, 0.1);
}

.line-remove {
  background: rgba(239, 68, 68, 0.08);
}

.line-context {
  background: transparent;
}

.line-header {
  background: rgba(0, 0, 0, 0.03);
}

.dark .line-add {
  background: rgba(34, 197, 94, 0.12);
}

.dark .line-remove {
  background: rgba(239, 68, 68, 0.1);
}

.dark .line-context {
  background: transparent;
}

.dark .line-header {
  background: rgba(255, 255, 255, 0.03);
}

/* ── 行号 ── */
.line-no {
  width: 38px;
  flex-shrink: 0;
  text-align: right;
  padding-right: 10px;
  font-size: 11px;
  color: #b0b0b0;
  user-select: none;
  position: relative;
  z-index: 1;
}

.dark .line-no {
  color: #484f58;
}

/* ── 行前缀（+/-/ ） ── */
.line-prefix {
  width: 14px;
  flex-shrink: 0;
  text-align: center;
  font-weight: 600;
  font-size: 11px;
  user-select: none;
  position: relative;
  z-index: 1;
}

.line-add .line-prefix { color: #16a34a; }
.line-remove .line-prefix { color: #dc2626; }
.line-context .line-prefix { color: transparent; }

.dark .line-add .line-prefix { color: #3fb950; }
.dark .line-remove .line-prefix { color: #f85149; }

/* ── 代码内容区 ── */
.line-content {
  flex: 1;
  min-width: 0;
  white-space: pre-wrap;
  word-break: break-all;
  overflow-wrap: break-word;
  padding-left: 4px;
  position: relative;
  z-index: 1;
  color: #1f2328;
}

.dark .line-content {
  color: #e6edf3;
}

/* highlight.js 渲染的 span 保持 diff 背景透明 */
.line-content :deep(span) {
  background: transparent !important;
}

/* ═══════════════════════════════════════════
   语法高亮配色 — 浅色模式（GitHub light 风格）
   ═══════════════════════════════════════════ */

/* 关键字：if/else/return/function/class/const/let/import/export 等 */
.line-content :deep(.hljs-keyword),
.line-content :deep(.hljs-selector-tag),
.line-content :deep(.hljs-literal) {
  color: #cf222e;
}

/* 字符串 */
.line-content :deep(.hljs-string),
.line-content :deep(.hljs-addition) {
  color: #0a3069;
}

/* 数字 */
.line-content :deep(.hljs-number) {
  color: #0550ae;
}

/* 注释 */
.line-content :deep(.hljs-comment),
.line-content :deep(.hljs-quote) {
  color: #6e7781;
  font-style: italic;
}

/* 函数名 / 类名 / 标题 */
.line-content :deep(.hljs-title),
.line-content :deep(.hljs-title.function_),
.line-content :deep(.hljs-title.class_),
.line-content :deep(.hljs-type) {
  color: #6639ba;
}

/* 内置函数 / 内置类型 */
.line-content :deep(.hljs-built_in),
.line-content :deep(.hljs-symbol) {
  color: #0550ae;
}

/* 模板字符串变量 */
.line-content :deep(.hljs-variable),
.line-content :deep(.hljs-template-variable) {
  color: #953800;
}

/* 正则表达式 */
.line-content :deep(.hljs-regexp) {
  color: #0a3069;
}

/* 属性 / 参数 */
.line-content :deep(.hljs-attr),
.line-content :deep(.hljs-attribute),
.line-content :deep(.hljs-params) {
  color: #8250df;
}

/* 标签名 */
.line-content :deep(.hljs-name) {
  color: #116329;
}

/* 元数据 / 装饰器 */
.line-content :deep(.hljs-meta) {
  color: #6e7781;
}

/* 删除 / 插入标记 */
.line-content :deep(.hljs-deletion) {
  color: #cf222e;
}

/* 选择器 */
.line-content :deep(.hljs-selector-id),
.line-content :deep(.hljs-selector-class),
.line-content :deep(.hljs-selector-attr),
.line-content :deep(.hljs-selector-pseudo) {
  color: #6639ba;
}

/* 强调 */
.line-content :deep(.hljs-emphasis) {
  font-style: italic;
}
.line-content :deep(.hljs-strong) {
  font-weight: 600;
}

/* ═══════════════════════════════════════════
   语法高亮配色 — 深色模式（GitHub dark 风格）
   ═══════════════════════════════════════════ */

.dark .line-content :deep(.hljs-keyword),
.dark .line-content :deep(.hljs-selector-tag),
.dark .line-content :deep(.hljs-literal) {
  color: #ff7b72;
}

.dark .line-content :deep(.hljs-string),
.dark .line-content :deep(.hljs-addition) {
  color: #a5d6ff;
}

.dark .line-content :deep(.hljs-number) {
  color: #79c0ff;
}

.dark .line-content :deep(.hljs-comment),
.dark .line-content :deep(.hljs-quote) {
  color: #8b949e;
  font-style: italic;
}

.dark .line-content :deep(.hljs-title),
.dark .line-content :deep(.hljs-title.function_),
.dark .line-content :deep(.hljs-title.class_),
.dark .line-content :deep(.hljs-type) {
  color: #d2a8ff;
}

.dark .line-content :deep(.hljs-built_in),
.dark .line-content :deep(.hljs-symbol) {
  color: #79c0ff;
}

.dark .line-content :deep(.hljs-variable),
.dark .line-content :deep(.hljs-template-variable) {
  color: #ffa657;
}

.dark .line-content :deep(.hljs-regexp) {
  color: #a5d6ff;
}

.dark .line-content :deep(.hljs-attr),
.dark .line-content :deep(.hljs-attribute),
.dark .line-content :deep(.hljs-params) {
  color: #d2a8ff;
}

.dark .line-content :deep(.hljs-name) {
  color: #7ee787;
}

.dark .line-content :deep(.hljs-meta) {
  color: #8b949e;
}

.dark .line-content :deep(.hljs-deletion) {
  color: #ff7b72;
}

.dark .line-content :deep(.hljs-selector-id),
.dark .line-content :deep(.hljs-selector-class),
.dark .line-content :deep(.hljs-selector-attr),
.dark .line-content :deep(.hljs-selector-pseudo) {
  color: #d2a8ff;
}

.dark .line-content :deep(.hljs-emphasis) {
  font-style: italic;
}
.dark .line-content :deep(.hljs-strong) {
  font-weight: 600;
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
  position: relative;
  z-index: 1;
}

.dark .line-comment-count {
  background: #818cf8;
}

/* ── 评论容器 ── */
.hunk-comments {
  display: contents;
}
</style>
