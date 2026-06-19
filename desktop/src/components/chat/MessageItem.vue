<script setup lang="ts">
import { computed, ref, watch } from "vue";
import StreamText from "./StreamText.vue";
import ToolCallGroup from "./ToolCallGroup.vue";
import ReasoningBlock from "./ReasoningBlock.vue";
import { useThemeStore } from "../../stores/theme";
import { groupToolCalls } from "../../utils/toolSummary";
import type { ChatMessage } from "../../stores/project";

const props = defineProps<{
  message: ChatMessage;
}>();

const themeStore = useThemeStore();
const isUser = computed(() => props.message.role === "user");
const isAssistant = computed(() => props.message.role === "assistant");
const hasToolCalls = computed(() => (props.message.toolCalls?.length ?? 0) > 0);

// ── Thinking / done state ───────────────────────────────────────────

/** Whether every tool call in this message has completed. */
const allToolsDone = computed(() => {
  if (!hasToolCalls.value) return true;
  return (props.message.toolCalls ?? []).every((tc) => tc.resultDone);
});

/** Tools are still executing — show the thinking indicator. */
const isThinking = computed(() => hasToolCalls.value && !allToolsDone.value);

/** User has clicked "Done, click to view…" to expand the summary. */
const showToolDetails = ref(false);

// Reset details visibility when a new batch of tools starts running
watch(isThinking, (thinking) => {
  if (thinking) {
    showToolDetails.value = false;
  }
});

// ── Tool call grouping ──────────────────────────────────────────────

const toolGroups = computed(() => {
  if (!hasToolCalls.value) return [];
  return groupToolCalls(props.message.toolCalls ?? []);
});
</script>

<template>
  <div
    class="message-item"
    :class="{ user: isUser, assistant: isAssistant, dark: themeStore.isDark }"
  >
    <!-- ── User message: right-aligned pill ── -->
    <template v-if="isUser">
      <div class="user-pill">
        <span class="user-pill-text">{{ message.content }}</span>
      </div>
    </template>

    <!-- ── AI message: thinking process at top, text below ── -->
    <template v-else-if="isAssistant">
      <div class="agent-block">
        <!-- ── LLM Reasoning / Thinking block ── -->
        <ReasoningBlock
          :reasoning="message.reasoning"
          :reasoning-done="message.reasoningDone"
          v-model="message.reasoningExpanded"
        />

        <!-- ── Thinking / Tool calls — TOP ── -->
        <div v-if="hasToolCalls" class="tool-calls-block">
          <!-- State 1: Thinking — tools still running -->
          <div v-if="isThinking" class="thinking-indicator">
            <span class="thinking-ring">
              <span class="thinking-ring-inner"></span>
            </span>
            <span class="thinking-text">Thinking, wait a moment...</span>
          </div>

          <!-- State 2: Done — persistent header, click to toggle details below -->
          <div v-else class="tool-done-block">
            <div
              class="thinking-done"
              :class="{ expanded: showToolDetails }"
              @click="showToolDetails = !showToolDetails"
            >
              <span class="done-icon">✓</span>
              <span class="done-text">
                {{ showToolDetails ? 'Done' : 'Done, click to view...' }}
              </span>
              <span class="done-chevron">{{ showToolDetails ? '▾' : '▸' }}</span>
            </div>

            <!-- Expandable: group headers, click to drill down -->
            <div v-if="showToolDetails" class="tool-done-details">
              <div class="tool-groups-list">
                <ToolCallGroup
                  v-for="group in toolGroups"
                  :key="group.category"
                  :group="group"
                />
              </div>
            </div>
          </div>
        </div>

        <!-- ── AI text response — BELOW ── -->
        <StreamText
          :text="message.content"
          :done="message.done"
        />
        <!-- Typing indicator when streaming empty -->
        <div
          v-if="!message.content && !message.done"
          class="typing-indicator"
        >
          <span class="dot"></span>
          <span class="dot"></span>
          <span class="dot"></span>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
/* ── Message container ── */
.message-item {
  display: flex;
  align-items: flex-start;
  margin-bottom: 36px;
  padding: 0;
}

.message-item.user {
  justify-content: flex-end;
}

.message-item.assistant {
  justify-content: flex-start;
}

/* ── User pill — minimal tag, no bubble ── */
.user-pill {
  max-width: 75%;
  background: #2E3036;
  padding: 10px 18px;
  border-radius: 10px;
  transition: background 0.2s ease;
}

.dark .user-pill {
  background: #2E3036;
}

.message-item:not(.dark) .user-pill {
  background: #e5e5e5;
}

.user-pill-text {
  font-size: 15px;
  line-height: 1.65;
  color: #e4e4e7;
  word-break: break-word;
  white-space: pre-wrap;
}

.message-item:not(.dark) .user-pill-text {
  color: #1e293b;
}

/* ── Agent block — full-width structured text ── */
.agent-block {
  width: 100%;
  padding: 0;
}

/* ── Tool calls block (now at top of message) ── */
.tool-calls-block {
  margin-bottom: 16px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

/* ── Thinking indicator (tools running) ── */
.thinking-indicator {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 0;
  user-select: none;
}

/* Spinning ring */
.thinking-ring {
  position: relative;
  width: 18px;
  height: 18px;
  flex-shrink: 0;
  border-radius: 50%;
  background: conic-gradient(
    rgba(99, 102, 241, 0.8) 0%,
    rgba(99, 102, 241, 0.3) 40%,
    rgba(128, 128, 128, 0.08) 60%,
    rgba(128, 128, 128, 0.08) 100%
  );
  animation: thinkSpin 1s linear infinite;
}

.thinking-ring-inner {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: #fafafa;
}

.dark .thinking-ring-inner {
  background: #1a1a20;
}

.thinking-text {
  font-size: 13px;
  font-weight: 460;
  color: #888;
  letter-spacing: 0.01em;
}

.dark .thinking-text {
  color: #9ca3af;
}

/* ── Done state (persistent header, click to toggle) ── */
.tool-done-block {
  display: flex;
  flex-direction: column;
}

.thinking-done {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 0;
  cursor: pointer;
  user-select: none;
  border-radius: 8px;
  transition: background 0.12s ease;
}

.thinking-done:hover {
  background: rgba(128, 128, 128, 0.04);
}

.done-icon {
  width: 18px;
  height: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background: #22c55e;
  color: #fff;
  font-size: 11px;
  font-weight: 700;
  flex-shrink: 0;
}

.done-text {
  font-size: 13px;
  font-weight: 460;
  color: #888;
  letter-spacing: 0.01em;
  transition: color 0.12s ease;
}

.thinking-done:hover .done-text {
  color: #aaa;
}

.done-chevron {
  margin-left: auto;
  font-size: 11px;
  color: #888;
  transition: transform 0.15s ease;
}

.dark .done-text {
  color: #9ca3af;
}

.dark .thinking-done:hover .done-text {
  color: #d1d5db;
}

.dark .thinking-done:hover {
  background: rgba(255, 255, 255, 0.04);
}

.dark .done-chevron {
  color: #9ca3af;
}

/* Expandable details below the Done header */
.tool-done-details {
  margin-top: 4px;
  padding-left: 4px;
  animation: doneDetailsIn 0.2s ease;
}

@keyframes doneDetailsIn {
  from {
    opacity: 0;
    transform: translateY(-4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes thinkSpin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

/* ── Summary chips bar ── */
.tool-summary-bar {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
  padding: 4px 0;
}

.summary-chip {
  display: inline-flex;
  align-items: center;
  padding: 3px 10px;
  border: 1px solid rgba(128, 128, 128, 0.15);
  border-radius: 12px;
  font-size: 12px;
  font-weight: 500;
  color: #888;
  background: rgba(128, 128, 128, 0.04);
  cursor: pointer;
  user-select: none;
  transition: all 0.15s ease;
}

.summary-chip:hover {
  background: rgba(128, 128, 128, 0.08);
  color: #aaa;
}

.summary-chip.active {
  color: #fff;
}

/* Category-specific active colors */
.summary-chip.chip-file_read.active {
  background: #3b82f6;
  border-color: #3b82f6;
  color: #fff;
}

.summary-chip.chip-file_write.active {
  background: #f97316;
  border-color: #f97316;
  color: #fff;
}

.summary-chip.chip-search.active {
  background: #22c55e;
  border-color: #22c55e;
  color: #fff;
}

.summary-chip.chip-directory.active {
  background: #a855f7;
  border-color: #a855f7;
  color: #fff;
}

.summary-chip.chip-shell.active {
  background: #6b7280;
  border-color: #6b7280;
  color: #fff;
}

.summary-chip.chip-web.active {
  background: #0ea5e9;
  border-color: #0ea5e9;
  color: #fff;
}

.summary-chip.chip-git.active {
  background: #ec4899;
  border-color: #ec4899;
  color: #fff;
}

.summary-chip.chip-plan.active {
  background: #eab308;
  border-color: #eab308;
  color: #1a1a1a;
}

/* Dark mode chip defaults */
.dark .summary-chip {
  border-color: rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.04);
  color: #9ca3af;
}

.dark .summary-chip:hover {
  background: rgba(255, 255, 255, 0.08);
  color: #d1d5db;
}

.dark .summary-chip.active {
  color: #fff;
}

/* Summary toggle button */
.summary-toggle {
  margin-left: auto;
  padding: 3px 8px;
  border: none;
  border-radius: 10px;
  font-size: 11px;
  font-weight: 460;
  color: #888;
  background: transparent;
  cursor: pointer;
  user-select: none;
  white-space: nowrap;
  transition: color 0.12s ease;
}

.summary-toggle:hover {
  color: #aaa;
}

.dark .summary-toggle {
  color: #9ca3af;
}

.dark .summary-toggle:hover {
  color: #d1d5db;
}

/* Tool groups list container */
.tool-groups-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

/* ── Typing indicator ── */
.typing-indicator {
  display: flex;
  gap: 6px;
  padding: 8px 0;
}

.dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: #6b7280;
  animation: blink 1.4s infinite both;
}

.dark .dot {
  background: #9ca3af;
}

.dot:nth-child(2) {
  animation-delay: 0.2s;
}

.dot:nth-child(3) {
  animation-delay: 0.4s;
}

@keyframes blink {
  0%, 80%, 100% {
    opacity: 0.3;
    transform: scale(0.8);
  }
  40% {
    opacity: 1;
    transform: scale(1);
  }
}
</style>
