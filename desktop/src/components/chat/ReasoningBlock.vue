<script setup lang="ts">
import { computed } from "vue";
import { useThemeStore } from "../../stores/theme";

const props = defineProps<{
  reasoning?: string;
  reasoningDone?: boolean;
  modelValue?: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: boolean];
}>();

const themeStore = useThemeStore();

/**
 * 思考块的展开/折叠状态。
 * 用户在流式输出期间手动点击的优先级最高（modelValue 已赋值）；
 * 未手动操作时：思考流式输出中默认展开，思考完成后默认收起，
 * 避免大段已完成的思考内容挤占回复区域的可见空间。
 */
const isExpanded = computed({
  get: () => {
    if (props.modelValue !== undefined) return props.modelValue;
    // 思考未完成 → 展开让用户实时观看；已完成 → 收起避免占位
    return isStreaming.value;
  },
  set: (value) => emit("update:modelValue", value),
});

/**
 * 判断思考内容是否仍在持续流式输出。
 * 当存在思考文本且尚未收到结束信号时保持动态思考态。
 */
const isStreaming = computed(() => {
  return !!props.reasoning && props.reasoningDone !== true;
});

/**
 * 判断思考过程是否已经结束。
 * 仅在收到完成标记后切换为静态查看入口。
 */
const isDone = computed(() => props.reasoningDone === true);

/**
 * 判断当前是否存在可展示的思考内容。
 * 通过思考文本是否为空决定是否允许展开查看。
 */
const hasContent = computed(() => !!props.reasoning && props.reasoning.length > 0);

/**
 * 切换思考内容面板的展开状态。
 * 仅在存在思考文本时响应点击并展开或收起详情。
 */
function toggle() {
  if (!hasContent.value) return;
  isExpanded.value = !isExpanded.value;
}
</script>

<template>
  <div
    v-if="hasContent || isStreaming"
    class="reasoning-section"
    :class="{ dark: themeStore.isDark, expanded: isExpanded }"
  >
    <button
      class="reasoning-status-button"
      :class="{ clickable: hasContent }"
      type="button"
      :disabled="!hasContent"
      @click="toggle"
    >
      <span class="reasoning-label">
        {{ isDone ? "思考已完成" : "思考中..." }}
      </span>
      <span v-if="hasContent" class="reasoning-chevron" :class="{ up: isExpanded }">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="6 9 12 15 18 9" />
        </svg>
      </span>
    </button>

    <div v-if="isExpanded && hasContent" class="reasoning-content">
      <pre class="reasoning-text">{{ reasoning }}</pre>
    </div>
  </div>
</template>

<style scoped>
.reasoning-section {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin: 0;
}

.reasoning-status-button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  width: fit-content;
  min-height: 36px;
  padding: 0;
  border: none;
  background: transparent;
  color: #9ca3af;
  font-size: 14px;
  line-height: 1.6;
  font-weight: 520;
  text-align: left;
  cursor: default;
  transition: color 0.15s ease;
}

.reasoning-status-button.clickable {
  cursor: pointer;
}

.reasoning-status-button.clickable:hover {
  color: #d1d5db;
}

.reasoning-label {
  letter-spacing: 0;
}

.reasoning-chevron {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: #6b7280;
  transition: transform 0.2s ease;
}

.reasoning-chevron.up {
  transform: rotate(180deg);
}

.reasoning-content {
  padding: 0;
  animation: reasoningFadeIn 0.2s ease;
}

.reasoning-text {
  margin: 0;
  padding: 0;
  background: transparent;
  font-family: "SF Mono", "Fira Code", "Cascadia Code", "JetBrains Mono", monospace;
  font-size: 12.5px;
  line-height: 1.7;
  color: #d6dce8;
  white-space: pre-wrap;
  word-break: break-word;
  overflow: visible;
}

.dark .reasoning-status-button {
  color: #9ca3af;
}

.dark .reasoning-status-button.clickable:hover {
  color: #d1d5db;
}

.dark .reasoning-chevron {
  color: #6b7280;
}

.dark .reasoning-text {
  color: #d6dce8;
}

@keyframes reasoningFadeIn {
  from {
    opacity: 0;
    transform: translateY(-4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
