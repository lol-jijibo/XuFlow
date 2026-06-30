<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import StreamText from "./StreamText.vue";
import ToolCallGroup from "./ToolCallGroup.vue";
import ReasoningBlock from "./ReasoningBlock.vue";
import { useThemeStore } from "../../stores/theme";
import { groupToolCalls } from "../../utils/toolSummary";
import type { ChatMessage } from "../../stores/project";

const props = defineProps<{
  message: ChatMessage;
  activeStreaming?: boolean;
}>();

const themeStore = useThemeStore();
const isUser = computed(() => props.message.role === "user");
const isAssistant = computed(() => props.message.role === "assistant");
const hasToolCalls = computed(() => (props.message.toolCalls?.length ?? 0) > 0);
const hasReasoning = computed(() => !!props.message.reasoning && props.message.reasoning.length > 0);
const reasoningPrompts = [
  "思考中...",
  "整理回答中...",
  "组织语言中...",
  "生成回复中...",
] as const;
const reasoningPromptIndex = ref(0);
let reasoningPromptTimer: ReturnType<typeof setInterval> | null = null;

/**
 * 判断当前消息中的工具调用是否全部结束。
 * 通过遍历所有工具结果状态，决定展示执行中还是完成摘要。
 */
const allToolsDone = computed(() => {
  if (!hasToolCalls.value) return true;
  return (props.message.toolCalls ?? []).every((tc) => tc.resultDone);
});

/**
 * 判断当前消息是否仍处于执行阶段。
 * 仅在最终回答尚未结束时维持统一的运行态提示。
 */
const isRunning = computed(() => props.activeStreaming === true && !props.message.done);

/**
 * 判断当前阶段是否正在调用工具。
 * 当工具调用已出现且仍有未返回结果的条目时进入该阶段。
 */
const isToolRunning = computed(() => hasToolCalls.value && !allToolsDone.value);

/**
 * 判断当前阶段是否正在组织最终回答。
 * 当工具已完成或无需工具但消息仍未结束时展示生成状态。
 */
const isPromptRotatingPhase = computed(() => isRunning.value && !isToolRunning.value);

const activeReasoningPrompt = computed(() => {
  return reasoningPrompts[reasoningPromptIndex.value];
});

/**
 * 生成当前消息顶部的统一状态文案。
 * 根据推理、工具调用和正文输出的先后阶段返回对应提示。
 */
const agentStatusText = computed(() => {
  if (isPromptRotatingPhase.value) return activeReasoningPrompt.value;
  if (isToolRunning.value) return "调用工具中...";
  return "";
});

function stopReasoningPromptRotation() {
  if (reasoningPromptTimer === null) return;
  clearInterval(reasoningPromptTimer);
  reasoningPromptTimer = null;
}

function startReasoningPromptRotation() {
  stopReasoningPromptRotation();
  reasoningPromptIndex.value = 0;
  reasoningPromptTimer = setInterval(() => {
    reasoningPromptIndex.value = (reasoningPromptIndex.value + 1) % reasoningPrompts.length;
  }, 1800);
}

/**
 * 记录工具完成明细的展开状态。
 * 用户点击完成状态后切换工具分组明细的展开与收起。
 */
const showToolDetails = ref(false);

/**
 * 判断是否需要展示工具完成状态行。
 * 仅在消息结束且确实发生过工具调用后显示完成提示。
 */
const showToolDoneStatus = computed(() => hasToolCalls.value && !isRunning.value);

/**
 * 判断是否需要展示运行中状态行。
 * 当消息仍在执行时将思考、工具和生成阶段统一展示在文本流顶部。
 */
const showRunningStatus = computed(() => !!agentStatusText.value);

/**
 * 按类别汇总工具调用结果。
 * 将原始工具调用列表整理成适合折叠展示的分组结构。
 */
const toolGroups = computed(() => {
  if (!hasToolCalls.value) return [];
  return groupToolCalls(props.message.toolCalls ?? []);
});

watch(isRunning, (running) => {
  if (running) {
    showToolDetails.value = false;
  }
});

watch(
  isPromptRotatingPhase,
  (isActive) => {
    if (isActive) {
      startReasoningPromptRotation();
      return;
    }

    stopReasoningPromptRotation();
  },
  { immediate: true }
);

onBeforeUnmount(() => {
  stopReasoningPromptRotation();
});
</script>

<template>
  <div
    class="message-item"
    :class="{ user: isUser, assistant: isAssistant, dark: themeStore.isDark }"
  >
    <template v-if="isUser">
      <div class="user-pill">
        <span class="user-pill-text">{{ message.content }}</span>
      </div>
    </template>

    <template v-else-if="isAssistant">
      <div class="assistant-message-row">
        <div class="assistant-avatar-column" aria-hidden="true">
          <span
            class="assistant-avatar-glyph"
            :class="{
              streaming: showRunningStatus,
              done: !showRunningStatus
            }"
          >
            <svg
              class="assistant-robot-svg"
              width="32"
              height="32"
              viewBox="0 0 32 32"
              fill="none"
              xmlns="http://www.w3.org/2000/svg"
            >
              <g class="assistant-glow">
                <circle cx="16" cy="17" r="12" />
              </g>
              <g class="assistant-antenna">
                <path class="assistant-antenna-line" d="M16 9V5.8" />
                <circle class="assistant-signal" cx="16" cy="4.5" r="2.6" />
              </g>
              <rect class="assistant-head" x="6.5" y="10" width="19" height="15" rx="6.5" />
              <rect class="assistant-face-shine" x="8.2" y="11.4" width="15.6" height="4.8" rx="2.4" />
              <circle class="assistant-eye assistant-eye-left" cx="12.4" cy="17.2" r="2" />
              <circle class="assistant-eye assistant-eye-right" cx="19.6" cy="17.2" r="2" />
              <path class="assistant-mouth" d="M13.4 21.1H18.6" />
              <g v-if="showRunningStatus" class="assistant-pulse-lines">
                <path class="assistant-pulse-line assistant-pulse-line-first" d="M26.4 15.4H30" />
                <path class="assistant-pulse-line assistant-pulse-line-second" d="M26.4 19H31" />
              </g>
              <g v-if="showRunningStatus" class="assistant-dot-flow">
                <circle class="assistant-dot assistant-dot-first" cx="12" cy="29" r="1.15" />
                <circle class="assistant-dot assistant-dot-second" cx="16" cy="29" r="1.15" />
                <circle class="assistant-dot assistant-dot-third" cx="20" cy="29" r="1.15" />
              </g>
            </svg>
          </span>
        </div>

        <div class="assistant-content-column">
          <div class="assistant-flow">
            <p v-if="showRunningStatus" class="assistant-flow-status" :class="{ 'shimmer-wave': isPromptRotatingPhase }">
              {{ agentStatusText }}
            </p>

            <ReasoningBlock
              v-if="hasReasoning"
              :reasoning="message.reasoning"
              :reasoning-done="message.reasoningDone"
              v-model="message.reasoningExpanded"
            />

            <div
              v-if="showToolDoneStatus"
              class="assistant-tool-status"
            >
              <button
                class="assistant-flow-status assistant-tool-toggle"
                type="button"
                @click="showToolDetails = !showToolDetails"
              >
                <span class="assistant-tool-check" aria-hidden="true">✓</span>
                <span>工具调用已完成</span>
                <span class="assistant-tool-chevron">{{ showToolDetails ? "▾" : "▸" }}</span>
              </button>

              <div v-if="showToolDetails" class="assistant-tool-details">
                <ToolCallGroup
                  v-for="group in toolGroups"
                  :key="group.category"
                  :group="group"
                />
              </div>
            </div>

            <StreamText
              v-if="message.content"
              :text="message.content"
              :done="message.done"
            />

            <div
              v-else-if="isRunning"
              class="typing-indicator"
            >
              <span class="dot"></span>
              <span class="dot"></span>
              <span class="dot"></span>
            </div>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
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

.user-pill {
  max-width: 75%;
  background: #2e3036;
  padding: 10px 18px;
  border-radius: 10px;
  transition: background 0.2s ease;
}

.dark .user-pill {
  background: #2e3036;
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

.assistant-message-row {
  width: 100%;
  display: flex;
  align-items: flex-start;
  gap: 10px;
}

.assistant-avatar-column {
  width: 36px;
  flex-shrink: 0;
  padding-top: 0;
}

.assistant-avatar-glyph {
  display: inline-flex;
  width: 36px;
  height: 36px;
  align-items: center;
  justify-content: center;
}

.assistant-content-column {
  min-width: 0;
  flex: 1;
}

.assistant-flow {
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-width: 0;
}

.assistant-flow-status {
  margin: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  line-height: 1.6;
  color: #9ca3af;
  font-weight: 520;
}

/* 波纹滑过英文提示词 — 渐变高光从左到右扫过文字，营造动态生成感 */
.shimmer-wave {
  background: linear-gradient(
    90deg,
    #9ca3af 0%,
    #9ca3af 35%,
    #d1d5db 50%,
    #9ca3af 65%,
    #9ca3af 100%
  );
  background-size: 200% 100%;
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
  animation: shimmerSweep 2.2s ease-in-out infinite;
}

.dark .shimmer-wave {
  background: linear-gradient(
    90deg,
    #9ca3af 0%,
    #9ca3af 35%,
    #e8e8ed 50%,
    #9ca3af 65%,
    #9ca3af 100%
  );
  background-size: 200% 100%;
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
  animation: shimmerSweep 2.2s ease-in-out infinite;
}

@keyframes shimmerSweep {
  0% {
    background-position: 200% 0;
  }
  100% {
    background-position: -200% 0;
  }
}

.assistant-tool-status {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.assistant-tool-toggle {
  padding: 0;
  border: none;
  background: transparent;
  text-align: left;
  cursor: pointer;
}

.assistant-tool-check {
  width: 16px;
  height: 16px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 999px;
  background: #22c55e;
  color: #ffffff;
  font-size: 10px;
  font-weight: 700;
  flex-shrink: 0;
}

.assistant-tool-chevron {
  color: #6b7280;
  font-size: 11px;
}

.assistant-tool-details {
  margin-left: 24px;
}

.assistant-robot-svg {
  display: block;
  width: 36px;
  height: 36px;
  overflow: visible;
}

.assistant-glow circle {
  fill: rgba(126, 144, 255, 0.08);
  filter: blur(4px);
}

.assistant-head {
  fill: rgba(31, 37, 49, 0.94);
  stroke: rgba(126, 144, 255, 0.62);
  stroke-width: 1.35;
}

.assistant-face-shine {
  fill: rgba(255, 255, 255, 0.045);
}

.assistant-eye {
  fill: #eef3ff;
}

.assistant-mouth {
  stroke: rgba(205, 216, 236, 0.86);
  stroke-width: 1.8;
  stroke-linecap: round;
}

.assistant-antenna-line {
  stroke: rgba(126, 144, 255, 0.76);
  stroke-width: 1.5;
  stroke-linecap: round;
}

.assistant-signal {
  fill: #86a4ff;
}

.assistant-pulse-lines {
  stroke-linecap: round;
}

.assistant-pulse-line {
  stroke: rgba(82, 216, 168, 0.78);
  stroke-width: 1.6;
  animation: assistantPulseSweep 1.4s ease-in-out infinite;
}

.assistant-pulse-line-second {
  animation-delay: 0.14s;
}

.assistant-dot-flow {
  fill: rgba(116, 223, 123, 0.82);
}

.assistant-dot {
  animation: assistantDotFlow 1.2s ease-in-out infinite;
  transform-box: fill-box;
  transform-origin: center;
}

.assistant-dot-second {
  animation-delay: 0.15s;
}

.assistant-dot-third {
  animation-delay: 0.3s;
}

.assistant-avatar-glyph.streaming .assistant-head {
  stroke: rgba(126, 144, 255, 0.78);
}

.assistant-avatar-glyph.streaming .assistant-signal {
  animation: assistantSignalGlow 1.6s ease-in-out infinite;
}

.assistant-avatar-glyph.done .assistant-head {
  stroke: rgba(57, 199, 109, 0.54);
  fill: rgba(29, 36, 44, 0.92);
}

.assistant-avatar-glyph.done .assistant-signal {
  fill: rgba(57, 199, 109, 0.82);
}

.assistant-avatar-glyph.done .assistant-mouth {
  stroke: rgba(234, 255, 240, 0.92);
}

.typing-indicator {
  display: flex;
  gap: 6px;
  padding: 4px 0 0;
}

.dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: #6b7280;
  animation: blink 1.4s infinite both;
}

.dark .assistant-flow-status {
  color: #9ca3af;
}

.dark .assistant-tool-chevron {
  color: #9ca3af;
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

@keyframes assistantSignalGlow {
  0%, 100% {
    transform: scale(0.92);
    opacity: 0.76;
  }
  50% {
    transform: scale(1.08);
    opacity: 1;
  }
}

@keyframes assistantPulseSweep {
  0%, 100% {
    opacity: 0.24;
    transform: translateX(-1px);
  }
  50% {
    opacity: 0.96;
    transform: translateX(1px);
  }
}

@keyframes assistantDotFlow {
  0%, 100% {
    opacity: 0.28;
    transform: translateY(0);
  }
  50% {
    opacity: 0.92;
    transform: translateY(-1px);
  }
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
