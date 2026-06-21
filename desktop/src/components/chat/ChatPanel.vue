<script setup lang="ts">
import { ref, nextTick, watch, onMounted, onUnmounted, computed } from "vue";
import { NInput, NScrollbar, NSelect, useMessage } from "naive-ui";
import MessageItem from "./MessageItem.vue";
import TodoPanel from "./TodoPanel.vue";
import PlanApprovalCard from "../approval/PlanApprovalCard.vue";
import { useAgentStore } from "../../stores/agent";
import { useThemeStore } from "../../stores/theme";
import { useConfigStore } from "../../stores/config";
import { useProjectStore } from "../../stores/project";
import { useTauriEvent } from "../../composables/useTauriEvent";

const store = useAgentStore();
const themeStore = useThemeStore();
const configStore = useConfigStore();
const msg = useMessage();
const { setupListeners, teardownListeners } = useTauriEvent();
const inputText = ref("");
const scrollRef = ref<InstanceType<typeof NScrollbar> | null>(null);
const sending = ref(false);

// ── Model switch animation ──
const modelSwitchAnim = ref(false);
let modelSwitchTimer: ReturnType<typeof setTimeout> | null = null;
watch(
  () => configStore.activeModelName,
  () => {
    modelSwitchAnim.value = true;
    if (modelSwitchTimer) clearTimeout(modelSwitchTimer);
    modelSwitchTimer = setTimeout(() => {
      modelSwitchAnim.value = false;
    }, 600);
  }
);
onUnmounted(() => {
  if (modelSwitchTimer) clearTimeout(modelSwitchTimer);
});

// ── Footbar state ──
const isInfoTooltipVisible = ref(false);
const autoExecute = ref(false);
// Plan Mode is sourced from the agent store so it persists across toggles
const planMode = computed({
  get: () => store.planMode,
  set: (v: boolean) => { store.planMode = v; },
});

// Token usage — sourced from agent store, updated in real time via Tauri events
const tokenPercent = computed(() => store.tokenUsagePercent);
const tokenWarningLevel = computed(() => store.tokenWarningLevel);
const tokenRemaining = computed(() => store.contextRemaining);
const tokenUsage = computed(() => store.tokenUsage);
const contextWindow = computed(() => store.contextWindow);

const isEmpty = computed(() => store.messages.length === 0);
const canSend = computed(() => inputText.value.trim().length > 0);

// ── Smooth wheel scrolling ──
function smoothWheelHandler(e: WheelEvent) {
  // Only smooth vertical scrolling; let horizontal/shift-wheel pass through
  if (Math.abs(e.deltaX) > Math.abs(e.deltaY)) return;

  e.preventDefault();
  const target = e.currentTarget as HTMLElement;
  target.scrollBy({
    top: e.deltaY,
    left: 0,
    behavior: "smooth",
  });
}

function attachSmoothScroll() {
  // Find the scrollable content container inside NScrollbar
  const container = scrollRef.value?.$el as HTMLElement | undefined;
  if (!container) return;
  const scrollContent = container.querySelector(".n-scrollbar-container") as HTMLElement | null;
  if (scrollContent) {
    scrollContent.addEventListener("wheel", smoothWheelHandler, { passive: false });
  }
}

onMounted(() => {
  setupListeners();
  // Push current credentials to the Rust backend on mount
  store.configureAgent();
  // Attach smooth scrolling once the scrollbar is rendered
  nextTick(() => attachSmoothScroll());
});

onUnmounted(() => {
  teardownListeners();
  // Clean up wheel listener
  const container = scrollRef.value?.$el as HTMLElement | undefined;
  if (container) {
    const scrollContent = container.querySelector(".n-scrollbar-container") as HTMLElement | null;
    if (scrollContent) {
      scrollContent.removeEventListener("wheel", smoothWheelHandler);
    }
  }
});

// Re-attach smooth scroll handler when the scrollbar appears (first message)
watch(isEmpty, (empty) => {
  if (!empty) {
    nextTick(() => attachSmoothScroll());
  }
});

// Re-sync backend whenever the user switches model or provider
watch(
  () => [configStore.activeModelId, configStore.activeApiKey] as const,
  () => {
    store.configureAgent();
  }
);

async function sendMessage() {
  const text = inputText.value.trim();
  if (!text || sending.value) return;

  // 发送前校验当前 provider 对应的 API 密钥是否已配置。
  // 如果为空则直接提示用户前往设置页配置，避免透传原始 HTTP 401 错误。
  // 检测逻辑：读取 configStore 中当前 provider 对应的 key 字段。
  if (!configStore.activeApiKey) {
    const providerName = configStore.activeProvider === "deepseek" ? "DeepSeek" : configStore.activeProvider === "kimi" ? "Kimi" : "火山引擎";
    msg.warning(`请先在设置中配置 ${providerName} API 密钥`);
    return;
  }

  // Validate that there is an active conversation before clearing input
  const projectStore = useProjectStore();
  if (!projectStore.activeConversation) {
    // Auto-recover: create a default project + conversation if everything is gone
    if (!projectStore.activeProject) {
      const project = projectStore.createProject("默认项目");
      projectStore.switchTo(project.id);
    }
    if (projectStore.activeProject && !projectStore.activeConversation) {
      const conv = projectStore.createConversation(projectStore.activeProject.id, "默认会话");
      projectStore.switchTo(projectStore.activeProject.id, conv.id);
    }
    if (!projectStore.activeConversation) {
      console.error("[chat] Failed to create conversation — cannot send message");
      return;
    }
  }

  sending.value = true;
  // Only clear input after we know the message will be processed
  inputText.value = "";

  try {
    await store.sendMessage(text);
  } catch (e) {
    // Restore the text on failure so the user can retry
    console.error("[chat] sendMessage failed:", e);
    inputText.value = text;
  } finally {
    sending.value = false;
    nextTick(() => {
      scrollRef.value?.scrollTo({ top: 99999, behavior: "smooth" });
    });
  }
}

function handleStop() {
  store.stopGeneration();
}

watch(
  () => store.messages.length,
  () => {
    nextTick(() => {
      scrollRef.value?.scrollTo({ top: 99999, behavior: "smooth" });
    });
  }
);
</script>

<template>
  <div class="chat-panel" :class="{ dark: themeStore.isDark }">
    <!-- Empty state / Welcome -->
    <div v-if="isEmpty" class="welcome-container">
      <div class="welcome-content">
        <div class="welcome-logo">
          <img src="/xuflow.png" alt="Xuflow" class="welcome-logo-img" />
        </div>
        <h2 class="welcome-title">开始对话</h2>
        <p class="welcome-subtitle">
          Xuflow 是你的 AI 编程助手，可以帮助你编写、调试和优化代码。
        </p>
      </div>
    </div>

    <!-- Chat body: keep the footer overlay full width while sharing one centered content width -->
    <div v-else class="chat-body">
      <NScrollbar ref="scrollRef" class="chat-scroll">
        <div class="chat-content-shell max-w-4xl mx-auto">
          <div class="message-list">
            <!-- Structured overlays: plan approval + todo list -->
            <PlanApprovalCard />
            <TodoPanel />
            <MessageItem
              v-for="(msg, i) in store.messages"
              :key="i"
              :message="msg"
            />
          </div>
        </div>
      </NScrollbar>
    </div> <!-- /chat-body -->

    <!-- Footer: full-width overlay + centered input card -->
    <div class="chat-footer-outer">
      <div class="chat-footer-shell chat-content-shell max-w-4xl mx-auto">
        <div
          class="chat-footer"
        >
            <!-- Input row: text area + send button side by side -->
            <div class="input-row">
              <div class="input-area">
                <NInput
                  :value="inputText"
                  @update:value="inputText = $event"
                  type="textarea"
                  placeholder="向Agent下达指令..."
                  :autosize="{ minRows: 1, maxRows: 6 }"
                  :disabled="store.isRunning"
                  @keydown.enter.exact.prevent="sendMessage"
                  class="chat-input"
                />
              </div>
              <!-- Send / Stop button -->
              <button
                v-if="store.isRunning"
                class="send-circle stop-circle"
                @click="handleStop"
                title="停止生成"
              >
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                  <circle cx="8" cy="8" r="5.25" fill="#ffffff"/>
                  <rect x="5.5" y="5.5" width="5" height="5" rx="0.8" fill="#2C2C2E"/>
                </svg>
              </button>
              <button
                v-else
                class="send-circle"
                :class="{ active: canSend && !sending }"
                :disabled="!canSend || sending"
                @click="sendMessage"
                title="发送"
              >
                <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
                  <path d="M10 16V5M6 9l4-4 4 4" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              </button>
            </div>

            <!-- ── Footbar: status strip attached below input ── -->
            <div
              class="chat-footbar"
            >
              <!-- Left: model selector — plain text label over transparent NSelect -->
              <div class="footbar-left">
                <div class="model-select-wrap">
                  <span class="model-name-label" :class="{ 'model-switch-pulse': modelSwitchAnim }">{{ configStore.activeModelName }}</span>
                  <svg width="10" height="10" viewBox="0 0 10 10" fill="none" class="model-chevron">
                    <path d="M2.5 3.5L5 6L7.5 3.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
                  </svg>
                  <NSelect
                    v-model:value="configStore.activeModelId"
                    :options="configStore.modelOptions"
                    size="small"
                    class="footbar-model-select"
                  />
                </div>
              </div>

              <!-- Right: context capacity + auto-execute -->
              <div class="footbar-right">
                <!-- Context capacity circle -->
                <div
                  class="context-circle-wrap"
                  @mouseenter="isInfoTooltipVisible = true"
                  @mouseleave="isInfoTooltipVisible = false"
                >
                  <svg width="15" height="15" viewBox="0 0 15 15" fill="none" class="context-circle-svg">
                    <!-- Background ring -->
                    <circle cx="7.5" cy="7.5" r="6.25" stroke="currentColor" stroke-width="1.2" opacity="0.35"/>
                    <!-- Foreground arc (token usage fill) — circumference ≈ 39.27 -->
                    <circle
                      v-if="tokenPercent > 0"
                      cx="7.5" cy="7.5" r="6.25"
                      fill="none"
                      :stroke="tokenWarningLevel === 'red' ? '#ef4444' : tokenWarningLevel === 'orange' ? '#f59e0b' : tokenWarningLevel === 'yellow' ? '#eab308' : '#22c55e'"
                      stroke-width="1.8"
                      :stroke-dasharray="((tokenPercent / 100) * 39.27) + ' ' + (39.27 - ((tokenPercent / 100) * 39.27))"
                      stroke-linecap="round"
                      transform="rotate(-90 7.5 7.5)"
                    />
                  </svg>
                  <!-- Trim badge: transient, fades out -->
                  <span
                    v-if="store.contextTrimmed"
                    class="trim-badge"
                    :class="{ 'trim-badge-hiding': !store.contextTrimmed }"
                  >对话已自动整理</span>
                  <div v-if="isInfoTooltipVisible" class="context-tooltip">
                    <span class="context-tooltip-text">
                      已用 {{ tokenPercent }}% · 剩余 {{ tokenRemaining.toLocaleString() }} tokens<br/>
                      <span class="context-tooltip-detail">
                        {{ tokenUsage.toLocaleString() }} / {{ contextWindow.toLocaleString() }}
                      </span>
                    </span>
                    <div class="context-tooltip-arrow"></div>
                  </div>
                </div>

                <div class="auto-execute-group">
                  <button
                    class="ios-toggle"
                    :class="{ active: autoExecute }"
                    @click="autoExecute = !autoExecute"
                    title="自动执行工具调用"
                  >
                    <span class="ios-toggle-knob"></span>
                  </button>
                  <span class="toggle-label">自动执行</span>
                </div>

                <div class="auto-execute-group">
                  <button
                    class="ios-toggle"
                    :class="{ active: planMode }"
                    @click="planMode = !planMode"
                    title="先规划再执行"
                  >
                    <span class="ios-toggle-knob"></span>
                  </button>
                  <span class="toggle-label">先规划</span>
                </div>
              </div>
            </div>
          </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.chat-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: #fafafa;
  transition: background-color 0.3s ease;
}

.chat-panel.dark {
  background: #1a1a20;
}

/* Welcome */
.welcome-container {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
}

.welcome-content {
  max-width: 600px;
  width: 100%;
  text-align: center;
}

.welcome-logo {
  margin-bottom: 24px;
}

.welcome-logo-img {
  width: 72px;
  height: 72px;
  border-radius: 18px;
  object-fit: contain;
}

.welcome-title {
  font-size: 28px;
  font-weight: 700;
  margin: 0 0 8px;
  color: #1c1c1c;
}

.chat-panel.dark .welcome-title {
  color: #ddd;
}

.welcome-subtitle {
  font-size: 15px;
  color: #777;
  margin: 0 0 32px;
  line-height: 1.6;
}

.chat-panel.dark .welcome-subtitle {
  color: #999;
}

/* Messages */
.chat-scroll {
  flex: 1;
  min-height: 0;
}

/* Smooth native scroll for the scrollable content area */
.chat-scroll :deep(.n-scrollbar-container),
.chat-scroll :deep(.n-scrollbar-content) {
  scroll-behavior: smooth;
}

/* Override Naive UI's native scroll rail — make it less abrupt */
.chat-scroll :deep(.n-scrollbar-rail) {
  transition: opacity 0.3s ease;
}

/* ── Shared body: keep scroll area flexible and footer full width ── */
.chat-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.chat-content-shell {
  width: 100%;
  max-width: 56rem;
  margin: 0 auto;
}

.message-list {
  padding: 44px 44px 52px;
}

/* ── Footer overlay: spans the panel, while the input card stays centered ── */
.chat-footer-outer {
  flex-shrink: 0;
  width: 100%;
  padding: 14px 24px 24px;
  background: linear-gradient(to top, rgba(250, 250, 250, 0.92) 0%, rgba(250, 250, 250, 0) 60%);
}

.chat-panel.dark .chat-footer-outer {
  background: linear-gradient(to top, rgba(26, 26, 32, 0.94) 0%, rgba(26, 26, 32, 0) 60%);
}

.chat-footer-shell {
  padding: 0 44px;
}

/* ── Apple-style frosted glass input + footbar ── */
.chat-footer {
  position: relative;
  padding: 18px 22px 0;
  width: 100%;
  background: rgba(255, 255, 255, 0.72);
  backdrop-filter: blur(24px) saturate(1.2);
  -webkit-backdrop-filter: blur(24px) saturate(1.2);
  border: 1px solid rgba(255, 255, 255, 0.3);
  border-radius: 28px;
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.04),
    0 2px 8px rgba(0, 0, 0, 0.02),
    inset 0 0 0 1px rgba(255, 255, 255, 0.5);
  transition: background 0.4s ease, border-color 0.4s ease, box-shadow 0.4s ease;
  font-family: -apple-system, BlinkMacSystemFont, "SF Pro Display", "SF Pro Text", "Helvetica Neue", sans-serif;
}

.chat-panel.dark .chat-footer {
  background: rgba(28, 28, 33, 0.82);
  backdrop-filter: blur(24px) saturate(1.1);
  -webkit-backdrop-filter: blur(24px) saturate(1.1);
  border-color: rgba(255, 255, 255, 0.08);
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.3),
    inset 0 0 0 1px rgba(255, 255, 255, 0.04);
}

/* ── Input area — pure white canvas ── */
.input-area {
  position: relative;
  flex: 1;
  min-width: 0;
}

.input-area :deep(.n-input__border),
.input-area :deep(.n-input__state-border) {
  display: none;
}

.input-area :deep(.n-input-wrapper) {
  padding: 0;
  background: transparent !important;
}

.input-area :deep(.n-input) {
  background: transparent !important;
}

.input-area :deep(.n-input__textarea) {
  background: transparent !important;
  padding: 0 !important;
}

.input-area :deep(.n-input__textarea-el) {
  resize: none;
  font-size: 14px;
  line-height: 1.5;
  font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Helvetica Neue", sans-serif;
  background: transparent !important;
  color: #1a1a1a;
  padding: 0;
  letter-spacing: -0.01em;
}

.input-area :deep(.n-input__placeholder) {
  font-size: 15px !important;
  line-height: 1.5 !important;
  padding: 0 !important;
}

.chat-panel.dark .input-area :deep(.n-input__textarea-el) {
  color: #ececf0;
}

.chat-panel.dark .input-area :deep(.n-input__placeholder) {
  color: #888 !important;
}

/* ── Input row: text area + send button side by side ── */
.input-row {
  display: flex;
  align-items: flex-start;
  gap: 10px;
}

/* Send circle — flat light-gray background + dark arrow, high contrast */
.send-circle {
  width: 38px;
  height: 38px;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 50%;
  background: #E5E7EB;
  color: #111827;
  cursor: pointer;
  transition: all 0.25s cubic-bezier(0.25, 0.1, 0.25, 1);
  flex-shrink: 0;
  box-shadow: none;
}

.send-circle:hover {
  background: #D1D5DB;
  color: #000000;
}

.send-circle.active {
  background: #D1D5DB;
  color: #000000;
  box-shadow: none;
}

.send-circle.active:hover {
  background: #9CA3AF;
  color: #000000;
  transform: scale(1.05);
  box-shadow: none;
}

.send-circle[disabled] {
  background: #F3F4F6;
  color: #D1D5DB;
  cursor: default;
  transform: none !important;
}

.chat-panel.dark .send-circle {
  background: #4B5563;
  color: #F3F4F6;
  box-shadow: none;
}

.chat-panel.dark .send-circle:hover {
  background: #6B7280;
  color: #FFFFFF;
}

.chat-panel.dark .send-circle.active {
  background: #6B7280;
  color: #FFFFFF;
  box-shadow: none;
}

.chat-panel.dark .send-circle.active:hover {
  background: #9CA3AF;
  color: #111827;
}

.chat-panel.dark .send-circle[disabled] {
  background: #374151;
  color: #6B7280;
}

/* Stop button — dark rounded rectangle, flat design */
.stop-circle {
  width: 38px;
  height: 38px;
  border-radius: 20px;
  background: #222222;
  color: transparent;
  box-shadow: none;
}

.stop-circle:hover {
  background: #333333;
  transform: scale(1.05);
}

.chat-panel.dark .stop-circle {
  background: #3a3a3e;
}

.chat-panel.dark .stop-circle:hover {
  background: #4a4a4e;
}

/* ── Footbar ─────────────────────────────────────── */

.chat-footbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 48px;
  padding: 0 16px;
  margin: 10px -20px 0;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 0 0 28px 28px;   /* match parent bottom corners */
  background: rgba(17, 17, 21, 0.42);
  backdrop-filter: blur(4px);
  -webkit-backdrop-filter: blur(4px);
  opacity: 1;
  user-select: none;
}

/* ── Left: model selector ───────── */

.footbar-left {
  display: flex;
  align-items: center;
  flex-shrink: 1;
}

/* ── Model select wrap: visible text + invisible NSelect overlay ── */

.model-select-wrap {
  position: relative;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  overflow: visible;
}

.model-name-label {
  font-size: 13.5px;
  font-weight: 500;
  color: #c8c8c8;
  white-space: nowrap;
  letter-spacing: 0.01em;
  transition: color 0.15s ease, opacity 0.15s ease, transform 0.15s ease;
  pointer-events: none;
}

/* Subtle pulse animation when model switches */
.model-switch-pulse {
  animation: modelSwitchPulse 0.6s cubic-bezier(0.25, 0.1, 0.25, 1);
}

@keyframes modelSwitchPulse {
  0% {
    opacity: 0.5;
    transform: translateY(3px);
  }
  30% {
    opacity: 1;
    transform: translateY(-1px);
  }
  60% {
    opacity: 1;
    transform: translateY(0);
  }
  100% {
    opacity: 1;
    transform: translateY(0);
  }
}

.model-select-wrap:hover .model-name-label {
  color: #e0e0e0;
}

/* Chevron */
.model-chevron {
  color: #6b7280;
  flex-shrink: 0;
  pointer-events: none;
  transition: transform 0.2s ease, color 0.15s ease;
}

.model-select-wrap:hover .model-chevron {
  color: #9ca3af;
  transform: rotate(180deg);
}

/* Invisible NSelect overlays the label — handles all click & dropdown logic */
.footbar-model-select {
  position: absolute;
  left: 0;
  top: 50%;
  transform: translateY(-50%);
  min-width: 210px;
  opacity: 0;
}

/* Let the selection fill the wider hit area so dropdown is wide enough */
.footbar-model-select :deep(.n-base-selection) {
  min-height: 24px;
}

/* Ensure the dropdown menu is wide enough for long model names */
.footbar-model-select :deep(.n-base-select-menu) {
  min-width: 220px !important;
  animation: dropdownFadeIn 0.2s cubic-bezier(0.25, 0.1, 0.25, 1);
}

@keyframes dropdownFadeIn {
  from {
    opacity: 0;
    transform: translateY(-6px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* Dark theme */
.chat-panel.dark .model-name-label {
  color: #d4d4d4;
}

.chat-panel.dark .model-select-wrap:hover .model-name-label {
  color: #f0f0f0;
}
.context-circle-wrap {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
}

/* Minimal ring — outline only, light gray */
.context-circle-svg {
  width: 15px;
  height: 15px;
  color: #9ca3af;
  transition: color 0.2s ease;
}

.context-circle-wrap:hover .context-circle-svg {
  color: #b0b7c3;
}

.chat-panel.dark .context-circle-svg {
  color: #6b7280;
}

.chat-panel.dark .context-circle-wrap:hover .context-circle-svg {
  color: #9ca3af;
}

/* ── Hover tooltip with downward arrow ────────── */

.context-tooltip {
  position: absolute;
  bottom: calc(100% + 10px);
  left: 50%;
  transform: translateX(-50%);
  white-space: nowrap;
  background: #333;
  color: #fff;
  font-size: 12px;
  font-weight: 460;
  line-height: 1.4;
  padding: 6px 12px;
  border-radius: 8px;
  z-index: 20;
  box-shadow: 0 4px 14px rgba(0, 0, 0, 0.25);
  animation: contextTooltipIn 0.18s cubic-bezier(0.25, 0.1, 0.25, 1);
  letter-spacing: 0.01em;
}

@keyframes contextTooltipIn {
  from {
    opacity: 0;
    transform: translateX(-50%) translateY(4px);
  }
  to {
    opacity: 1;
    transform: translateX(-50%) translateY(0);
  }
}

/* Tooltip detail line (smaller, muted) */
.context-tooltip-detail {
  font-size: 10px;
  color: #999;
  font-weight: 400;
}

/* ── Context trim badge (transient, non-intrusive) ── */
.trim-badge {
  position: absolute;
  top: -6px;
  left: 50%;
  transform: translate(-50%, -100%);
  font-size: 10px;
  font-weight: 500;
  color: #f59e0b;
  white-space: nowrap;
  background: rgba(245, 158, 11, 0.1);
  padding: 2px 8px;
  border-radius: 6px;
  pointer-events: none;
  animation: trimBadgeIn 0.3s ease, trimBadgeOut 0.5s ease 2.5s forwards;
  z-index: 21;
}

@keyframes trimBadgeIn {
  from { opacity: 0; transform: translate(-50%, calc(-100% + 4px)); }
  to   { opacity: 1; transform: translate(-50%, -100%); }
}

@keyframes trimBadgeOut {
  from { opacity: 1; }
  to   { opacity: 0; }
}

/* Downward triangle arrow connecting tooltip to the circle */
.context-tooltip-arrow {
  position: absolute;
  top: 100%;
  left: 50%;
  transform: translateX(-50%);
  width: 0;
  height: 0;
  border-left: 5px solid transparent;
  border-right: 5px solid transparent;
  border-top: 5px solid #333;
}

/* ── Right: status + toggles ────────────────────── */

.footbar-right {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
}

.auto-execute-group,
.status-group {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

/* iOS-style mini toggle switch */
.ios-toggle {
  position: relative;
  width: 28px;
  height: 16px;
  padding: 0;
  border: none;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.18);
  cursor: pointer;
  transition: background 0.25s cubic-bezier(0.25, 0.1, 0.25, 1);
  flex-shrink: 0;
  outline: none;
}

.ios-toggle.active {
  background: #34c759;
}

.chat-panel.dark .ios-toggle {
  background: rgba(255, 255, 255, 0.18);
}

.chat-panel.dark .ios-toggle.active {
  background: #30d158;
}

.ios-toggle-knob {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: #fff;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15), 0 0 0 1px rgba(0, 0, 0, 0.04);
  transition: transform 0.25s cubic-bezier(0.25, 0.1, 0.25, 1);
}

.ios-toggle.active .ios-toggle-knob {
  transform: translateX(12px);
}

/* Toggle label */
.toggle-label {
  font-size: 12.5px;
  font-weight: 460;
  color: #c8c8c8;
  white-space: nowrap;
  letter-spacing: 0.01em;
}

.chat-panel.dark .toggle-label {
  color: #c8c8c8;
}

/* Subtle vertical divider */
.footbar-divider {
  width: 1px;
  height: 15px;
  background: rgba(255, 255, 255, 0.13);
  flex-shrink: 0;
}

.chat-panel.dark .footbar-divider {
  background: rgba(255, 255, 255, 0.08);
}

/* Status dot */
.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #94a3b8;
  flex-shrink: 0;
  transition: background 0.3s ease, box-shadow 0.3s ease;
}

.status-dot.running {
  background: #22c55e;
  box-shadow: 0 0 5px rgba(34, 197, 94, 0.5);
  animation: statusPulse 2s infinite;
}

@keyframes statusPulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

/* Status text */
.status-text {
  font-size: 12.5px;
  font-weight: 460;
  color: #c8c8c8;
  white-space: nowrap;
  letter-spacing: 0.01em;
}

.chat-panel.dark .status-text {
  color: #9ca3af;
}

@media (max-width: 768px) {
  .message-list {
    padding: 32px 22px 40px;
  }

  .chat-footer-outer {
    padding: 12px 14px 16px;
  }

  .chat-footer-shell {
    padding: 0 10px;
  }

  .chat-footbar {
    height: auto;
    min-height: 34px;
    gap: 8px;
    padding: 8px 14px;
    flex-wrap: wrap;
  }

  .footbar-right {
    margin-left: auto;
  }
}
</style>
