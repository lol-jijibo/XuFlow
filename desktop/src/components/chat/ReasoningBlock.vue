<script setup lang="ts">
import { computed } from "vue";
import { useThemeStore } from "../../stores/theme";

const props = defineProps<{
  reasoning?: string;
  reasoningDone?: boolean;
  modelValue?: boolean; // expanded state (v-model)
}>();

const emit = defineEmits<{
  "update:modelValue": [value: boolean];
}>();

const themeStore = useThemeStore();

const isExpanded = computed({
  get: () => props.modelValue ?? false,
  set: (v) => emit("update:modelValue", v),
});

const isStreaming = computed(() => {
  // Streaming if we have reasoning content but it's not marked done yet
  return !!props.reasoning && props.reasoningDone !== true;
});

const isDone = computed(() => props.reasoningDone === true);
const hasContent = computed(() => !!props.reasoning && props.reasoning.length > 0);

function toggle() {
  if (!hasContent.value) return;
  isExpanded.value = !isExpanded.value;
}
</script>

<template>
  <div
    v-if="hasContent || isStreaming"
    class="reasoning-block"
    :class="{ dark: themeStore.isDark, expanded: isExpanded }"
  >
    <!-- Collapsed header: clickable toggle -->
    <button
      class="reasoning-header"
      :class="{ clickable: hasContent }"
      @click="toggle"
      :disabled="!hasContent"
    >
      <!-- Streaming state: animated dots + 思考中... -->
      <template v-if="isStreaming">
        <span class="reasoning-icon spinning">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/>
          </svg>
        </span>
        <span class="reasoning-label">思考中</span>
        <span class="thinking-dots">
          <span class="dot"></span>
          <span class="dot"></span>
          <span class="dot"></span>
        </span>
      </template>

      <!-- Done state: click to view -->
      <template v-else-if="isDone">
        <span class="reasoning-icon done">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="20 6 9 17 4 12"/>
          </svg>
        </span>
        <span class="reasoning-label">Done</span>
        <span v-if="!isExpanded" class="reasoning-hint">, click to view...</span>
        <span v-else class="reasoning-hint">, click to collapse</span>
      </template>

      <!-- Expand/collapse chevron -->
      <span v-if="hasContent" class="reasoning-chevron" :class="{ up: isExpanded }">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="6 9 12 15 18 9"/>
        </svg>
      </span>
    </button>

    <!-- Expanded content -->
    <div v-if="isExpanded && hasContent" class="reasoning-content">
      <pre class="reasoning-text">{{ reasoning }}</pre>
    </div>
  </div>
</template>

<style scoped>
.reasoning-block {
  display: flex;
  flex-direction: column;
  margin: 8px 0 12px;
  border-radius: 10px;
  overflow: hidden;
  border: 1px solid rgba(128, 128, 128, 0.12);
  background: rgba(128, 128, 128, 0.04);
  transition: border-color 0.2s ease, background 0.2s ease;
}

.reasoning-block.dark {
  border-color: rgba(255, 255, 255, 0.06);
  background: rgba(255, 255, 255, 0.03);
}

.reasoning-block.expanded {
  border-color: rgba(128, 128, 128, 0.18);
}

.reasoning-block.dark.expanded {
  border-color: rgba(255, 255, 255, 0.1);
}

/* ── Header ── */
.reasoning-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 14px;
  border: none;
  background: transparent;
  font-size: 13px;
  font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Helvetica Neue", sans-serif;
  color: #888;
  cursor: default;
  width: 100%;
  text-align: left;
  transition: color 0.15s ease, background 0.15s ease;
}

.reasoning-header.clickable {
  cursor: pointer;
}

.reasoning-header.clickable:hover {
  background: rgba(128, 128, 128, 0.06);
  color: #aaa;
}

.reasoning-block.dark .reasoning-header {
  color: #9ca3af;
}

.reasoning-block.dark .reasoning-header.clickable:hover {
  background: rgba(255, 255, 255, 0.04);
  color: #d1d5db;
}

/* ── Icon ── */
.reasoning-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  color: #a0a0a0;
}

.reasoning-icon.done {
  color: #22c55e;
}

.reasoning-block.dark .reasoning-icon {
  color: #6b7280;
}

.reasoning-block.dark .reasoning-icon.done {
  color: #34d399;
}

.spinning {
  animation: spin 2s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

/* ── Label ── */
.reasoning-label {
  font-weight: 500;
  letter-spacing: -0.01em;
}

/* ── Thinking dots ── */
.thinking-dots {
  display: flex;
  align-items: center;
  gap: 3px;
}

.thinking-dots .dot {
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: #a0a0a0;
  animation: thinkingBlink 1.4s infinite both;
}

.reasoning-block.dark .thinking-dots .dot {
  background: #6b7280;
}

.thinking-dots .dot:nth-child(2) {
  animation-delay: 0.2s;
}

.thinking-dots .dot:nth-child(3) {
  animation-delay: 0.4s;
}

@keyframes thinkingBlink {
  0%, 80%, 100% { opacity: 0.3; transform: scale(0.8); }
  40% { opacity: 1; transform: scale(1); }
}

/* ── Hint text ── */
.reasoning-hint {
  opacity: 0.7;
  font-weight: 400;
}

/* ── Chevron ── */
.reasoning-chevron {
  margin-left: auto;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: transform 0.2s ease;
  color: #888;
}

.reasoning-chevron.up {
  transform: rotate(180deg);
}

.reasoning-block.dark .reasoning-chevron {
  color: #6b7280;
}

/* ── Expanded content ── */
.reasoning-content {
  padding: 0 14px 12px;
  animation: reasoningFadeIn 0.2s ease;
}

@keyframes reasoningFadeIn {
  from { opacity: 0; transform: translateY(-4px); }
  to { opacity: 1; transform: translateY(0); }
}

.reasoning-text {
  margin: 0;
  padding: 10px 14px;
  background: rgba(0, 0, 0, 0.04);
  border-radius: 8px;
  font-family: "SF Mono", "Fira Code", "Cascadia Code", "JetBrains Mono", monospace;
  font-size: 12.5px;
  line-height: 1.7;
  color: #6b7280;
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 400px;
  overflow-y: auto;
}

.reasoning-block.dark .reasoning-text {
  background: rgba(255, 255, 255, 0.04);
  color: #9ca3af;
}

/* Scrollbar styling for reasoning text */
.reasoning-text::-webkit-scrollbar {
  width: 6px;
}

.reasoning-text::-webkit-scrollbar-track {
  background: transparent;
}

.reasoning-text::-webkit-scrollbar-thumb {
  background: rgba(128, 128, 128, 0.2);
  border-radius: 3px;
}

.reasoning-text::-webkit-scrollbar-thumb:hover {
  background: rgba(128, 128, 128, 0.3);
}
</style>
