<script setup lang="ts">
import { ref } from "vue";
import { useReviewStore } from "../../stores/review";
import { useThemeStore } from "../../stores/theme";

// 行级评论输入框，悬停在 diff 行上时通过 "+" 按钮唤起

const props = defineProps<{
  filePath: string;
  lineContent: string;
}>();

const emit = defineEmits<{
  (e: "close"): void;
}>();

const store = useReviewStore();
const themeStore = useThemeStore();

const commentText = ref("");
const submitting = ref(false);

async function submitComment() {
  const text = commentText.value.trim();
  if (!text) return;

  submitting.value = true;
  try {
    store.addComment(props.filePath, props.lineContent, {
      author: "user",
      content: text,
    });
    commentText.value = "";
    emit("close");
  } catch (e) {
    console.error("[review] submitComment error:", e);
  } finally {
    submitting.value = false;
  }
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
    e.preventDefault();
    submitComment();
  } else if (e.key === "Escape") {
    emit("close");
  }
}
</script>

<template>
  <div class="comment-input-overlay" :class="{ dark: themeStore.isDark }" @click.self="emit('close')">
    <div class="comment-input-card">
      <div class="comment-input-header">
        <span class="comment-input-title">添加评论</span>
        <span class="comment-input-file">{{ filePath }}</span>
        <button class="comment-input-close" @click="emit('close')">
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
            <path d="M2 2l8 8M10 2l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
      </div>
      <textarea
        v-model="commentText"
        class="comment-textarea"
        placeholder="输入评论内容...（Ctrl+Enter 提交）"
        rows="3"
        @keydown="handleKeydown"
        autofocus
      />
      <div class="comment-input-footer">
        <span class="comment-hint">Ctrl+Enter 提交 · Esc 取消</span>
        <button
          class="comment-submit-btn"
          :disabled="!commentText.trim() || submitting"
          @click="submitComment"
        >
          {{ submitting ? "提交中..." : "提交评论" }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.comment-input-overlay {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.2);
  animation: overlayIn 0.12s ease;
}

@keyframes overlayIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

.comment-input-card {
  width: 420px;
  max-width: 90vw;
  background: #fff;
  border-radius: 12px;
  box-shadow: 0 8px 30px rgba(0, 0, 0, 0.15);
  padding: 16px;
  animation: cardIn 0.15s cubic-bezier(0.25, 0.1, 0.25, 1);
}

@keyframes cardIn {
  from { transform: scale(0.95); opacity: 0; }
  to { transform: scale(1); opacity: 1; }
}

.dark .comment-input-card {
  background: #1c1c1f;
  box-shadow: 0 8px 30px rgba(0, 0, 0, 0.4);
}

.comment-input-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 10px;
}

.comment-input-title {
  font-size: 13px;
  font-weight: 600;
  color: #1f2937;
}

.dark .comment-input-title {
  color: #e5e7eb;
}

.comment-input-file {
  font-size: 11px;
  color: #9ca3af;
  font-family: "SF Mono", "Cascadia Code", monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.comment-input-close {
  margin-left: auto;
  width: 22px;
  height: 22px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 5px;
  background: transparent;
  color: #9ca3af;
  cursor: pointer;
}

.comment-input-close:hover {
  background: rgba(0, 0, 0, 0.05);
  color: #374151;
}

.dark .comment-input-close:hover {
  background: rgba(255, 255, 255, 0.06);
  color: #d1d5db;
}

.comment-textarea {
  width: 100%;
  padding: 10px;
  border: 1px solid rgba(0, 0, 0, 0.1);
  border-radius: 8px;
  font-size: 13px;
  font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif;
  line-height: 1.5;
  resize: vertical;
  min-height: 60px;
  background: #fafafa;
  color: #1f2937;
  outline: none;
  transition: border-color 0.15s ease;
}

.comment-textarea:focus {
  border-color: #6366f1;
  background: #fff;
}

.dark .comment-textarea {
  background: #141417;
  border-color: rgba(255, 255, 255, 0.08);
  color: #e5e7eb;
}

.dark .comment-textarea:focus {
  border-color: #818cf8;
  background: #141417;
}

.comment-input-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 10px;
}

.comment-hint {
  font-size: 11px;
  color: #9ca3af;
}

.dark .comment-hint {
  color: #6b7280;
}

.comment-submit-btn {
  padding: 6px 14px;
  border: none;
  border-radius: 7px;
  background: #6366f1;
  color: #fff;
  font-size: 12px;
  font-weight: 550;
  cursor: pointer;
  transition: all 0.15s ease;
}

.comment-submit-btn:hover:not(:disabled) {
  background: #4f46e5;
}

.comment-submit-btn:disabled {
  opacity: 0.4;
  cursor: default;
}

.dark .comment-submit-btn {
  background: #818cf8;
}

.dark .comment-submit-btn:hover:not(:disabled) {
  background: #6366f1;
}
</style>
