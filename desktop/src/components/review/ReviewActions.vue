<script setup lang="ts">
import { computed } from "vue";
import { useReviewStore } from "../../stores/review";
import { useThemeStore } from "../../stores/theme";

// 审查面板底部操作栏：AI 审查触发、批量暂存/回退、提交

const store = useReviewStore();
const themeStore = useThemeStore();

const hasChanges = computed(() => store.diffFiles.length > 0);

async function handleAIReview() {
  store.reviewing = true;
  try {
    const prompt = store.buildReviewPrompt();
    // 将审查提示词作为消息发送给 Agent
    const { useAgentStore } = await import("../../stores/agent");
    const agentStore = useAgentStore();

    // 构建带有 diff 上下文的完整审查消息
    const diffContent = store.diffFiles
      .map((f) => `--- a/${f.path}\n+++ b/${f.path}\n` + f.hunks.map((h) => h.header + "\n" + h.lines.map((l) => (l.type === "add" ? "+" : l.type === "remove" ? "-" : " ") + l.content).join("\n")).join("\n"))
      .join("\n\n");

    await agentStore.sendMessage(`${prompt}\n\n以下是完整的 diff 内容：\n\`\`\`diff\n${diffContent}\n\`\`\``);
  } catch (e) {
    console.error("[review] AI review error:", e);
  } finally {
    store.reviewing = false;
  }
}
</script>

<template>
  <div class="review-actions" :class="{ dark: themeStore.isDark }">
    <!-- AI 审查按钮 -->
    <button
      class="action-btn action-btn-review"
      :disabled="!hasChanges || store.reviewing"
      @click="handleAIReview"
    >
      <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
        <path d="M7 2v8M3 7l4 4 4-4" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
      <span>{{ store.reviewing ? "审查中..." : "AI 审查" }}</span>
    </button>

    <!-- 批量操作 -->
    <div class="action-group">
      <button
        class="action-btn action-btn-stage"
        :disabled="!hasChanges"
        @click="store.stageAll()"
        title="暂存所有变更"
      >
        暂存全部
      </button>
      <button
        class="action-btn action-btn-revert"
        :disabled="!hasChanges"
        @click="store.revertAll()"
        title="回退所有变更"
      >
        回退全部
      </button>
    </div>
  </div>
</template>

<style scoped>
.review-actions {
  padding: 10px 14px;
  border-top: 1px solid rgba(0, 0, 0, 0.05);
  display: flex;
  flex-direction: column;
  gap: 8px;
  flex-shrink: 0;
}

.dark .review-actions {
  border-top-color: rgba(255, 255, 255, 0.05);
}

/* ── AI 审查按钮 ── */
.action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 8px 14px;
  border: none;
  border-radius: 8px;
  font-size: 12.5px;
  font-weight: 550;
  cursor: pointer;
  transition: all 0.15s ease;
  width: 100%;
}

.action-btn:disabled {
  opacity: 0.4;
  cursor: default;
}

.action-btn-review {
  background: #6366f1;
  color: #fff;
}

.action-btn-review:hover:not(:disabled) {
  background: #4f46e5;
  box-shadow: 0 2px 8px rgba(99, 102, 241, 0.3);
}

.dark .action-btn-review {
  background: #818cf8;
}

.dark .action-btn-review:hover:not(:disabled) {
  background: #6366f1;
}

/* ── 批量操作组 ── */
.action-group {
  display: flex;
  gap: 8px;
}

.action-btn-stage {
  flex: 1;
  background: rgba(34, 197, 94, 0.1);
  color: #15803d;
}

.action-btn-stage:hover:not(:disabled) {
  background: rgba(34, 197, 94, 0.2);
}

.dark .action-btn-stage {
  background: rgba(34, 197, 94, 0.12);
  color: #4ade80;
}

.dark .action-btn-stage:hover:not(:disabled) {
  background: rgba(34, 197, 94, 0.22);
}

.action-btn-revert {
  flex: 1;
  background: rgba(239, 68, 68, 0.08);
  color: #b91c1c;
}

.action-btn-revert:hover:not(:disabled) {
  background: rgba(239, 68, 68, 0.16);
}

.dark .action-btn-revert {
  background: rgba(239, 68, 68, 0.1);
  color: #f87171;
}

.dark .action-btn-revert:hover:not(:disabled) {
  background: rgba(239, 68, 68, 0.2);
}
</style>
