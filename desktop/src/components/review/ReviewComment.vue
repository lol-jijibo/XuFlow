<script setup lang="ts">
import type { ReviewComment } from "../../utils/diffParser";

// 单条行级审查评论

defineProps<{
  comment: ReviewComment;
  filePath: string;
  lineContent: string;
}>();

function severityLabel(severity?: string): string {
  switch (severity) {
    case "error": return "严重";
    case "warning": return "警告";
    case "info": return "提示";
    default: return "";
  }
}

function severityClass(severity?: string): string {
  switch (severity) {
    case "error": return "sev-error";
    case "warning": return "sev-warning";
    case "info": return "sev-info";
    default: return "";
  }
}
</script>

<template>
  <div class="review-comment" :class="{ 'comment-agent': comment.author === 'agent' }">
    <div class="comment-header">
      <span class="comment-author">
        {{ comment.author === "agent" ? "🤖 AI 审查" : "💬 你的评论" }}
      </span>
      <span v-if="comment.severity" class="comment-severity" :class="severityClass(comment.severity)">
        {{ severityLabel(comment.severity) }}
      </span>
      <span v-if="comment.category" class="comment-category">{{ comment.category }}</span>
    </div>
    <div class="comment-body">{{ comment.content }}</div>
    <div v-if="comment.suggestion" class="comment-suggestion">
      <span class="suggestion-label">修复建议：</span>
      <code class="suggestion-code">{{ comment.suggestion }}</code>
    </div>
  </div>
</template>

<style scoped>
.review-comment {
  margin: 4px 8px 4px 48px;
  padding: 8px 10px;
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.02);
  border: 1px solid rgba(0, 0, 0, 0.05);
  font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif;
}

.comment-agent {
  border-color: rgba(99, 102, 241, 0.15);
  background: rgba(99, 102, 241, 0.03);
}

.dark .review-comment {
  background: rgba(255, 255, 255, 0.02);
  border-color: rgba(255, 255, 255, 0.05);
}

.dark .comment-agent {
  border-color: rgba(129, 140, 248, 0.15);
  background: rgba(129, 140, 248, 0.04);
}

.comment-header {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 4px;
}

.comment-author {
  font-size: 11px;
  font-weight: 600;
  color: #374151;
}

.dark .comment-author {
  color: #e2e8f0;
}

.comment-severity {
  font-size: 10px;
  font-weight: 600;
  padding: 1px 5px;
  border-radius: 3px;
}

.sev-error {
  background: rgba(239, 68, 68, 0.12);
  color: #dc2626;
}
.sev-warning {
  background: rgba(234, 179, 8, 0.12);
  color: #b45309;
}
.sev-info {
  background: rgba(59, 130, 246, 0.12);
  color: #2563eb;
}

.comment-category {
  font-size: 10px;
  color: #9ca3af;
  background: rgba(0, 0, 0, 0.04);
  padding: 1px 5px;
  border-radius: 3px;
}

.dark .comment-category {
  color: #6b7280;
  background: rgba(255, 255, 255, 0.04);
}

.comment-body {
  font-size: 12px;
  color: #4b5563;
  line-height: 1.45;
}

.dark .comment-body {
  color: #d1d5db;
}

.comment-suggestion {
  margin-top: 6px;
  padding: 6px 8px;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.03);
  border-left: 2px solid #6366f1;
}

.dark .comment-suggestion {
  background: rgba(255, 255, 255, 0.03);
  border-left-color: #818cf8;
}

.suggestion-label {
  font-size: 10px;
  font-weight: 600;
  color: #6366f1;
}

.dark .suggestion-label {
  color: #818cf8;
}

.suggestion-code {
  font-size: 11px;
  font-family: "SF Mono", "Cascadia Code", monospace;
  color: #374151;
  display: block;
  margin-top: 2px;
  white-space: pre-wrap;
  word-break: break-all;
}

.dark .suggestion-code {
  color: #e2e8f0;
}
</style>
