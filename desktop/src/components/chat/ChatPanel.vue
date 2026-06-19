<script setup lang="ts">
import { ref, nextTick, watch, onMounted, onUnmounted, computed } from "vue";
import { NInput, NScrollbar } from "naive-ui";
import MessageItem from "./MessageItem.vue";
import TodoPanel from "./TodoPanel.vue";
import PlanApprovalCard from "../approval/PlanApprovalCard.vue";
import { useAgentStore } from "../../stores/agent";
import { useThemeStore } from "../../stores/theme";
import { useConfigStore, ALL_MODELS } from "../../stores/config";
import { useProjectStore } from "../../stores/project";
import { useTauriEvent } from "../../composables/useTauriEvent";

const store = useAgentStore();
const themeStore = useThemeStore();
const configStore = useConfigStore();
const { setupListeners, teardownListeners } = useTauriEvent();
const inputText = ref("");
const scrollRef = ref<InstanceType<typeof NScrollbar> | null>(null);
const sending = ref(false);

// ── Footbar state ──
const isInfoTooltipVisible = ref(false);
const isModelDropdownVisible = ref(false);
const autoExecute = ref(false);
// Plan Mode is sourced from the agent store so it persists across toggles
const planMode = computed({
  get: () => store.planMode,
  set: (v: boolean) => { store.planMode = v; },
});

// Grouped model list for the hover dropdown
const modelGroups = computed(() => {
  const deepseekOfficial = ALL_MODELS.filter(m => m.provider === "deepseek");
  const volcDeepseek = ALL_MODELS.filter(m => m.provider === "volcengine" && m.label.startsWith("DeepSeek"));
  const volcDoubao = ALL_MODELS.filter(m => m.provider === "volcengine" && m.label.startsWith("Doubao"));
  const volcGlm = ALL_MODELS.filter(m => m.provider === "volcengine" && m.label.startsWith("GLM"));
  return [
    { label: "DeepSeek 官方", models: deepseekOfficial },
    { label: "火山引擎 · DeepSeek 系列", models: volcDeepseek },
    { label: "火山引擎 · 豆包系列", models: volcDoubao },
    { label: "火山引擎 · GLM 系列", models: volcGlm },
  ].filter(g => g.models.length > 0);
});

function selectModel(modelValue: string) {
  configStore.setActiveModelId(modelValue);
  isModelDropdownVisible.value = false;
}

// Token usage — sourced from agent store, updated in real time via Tauri events
const tokenPercent = computed(() => store.tokenUsagePercent);
const tokenWarningLevel = computed(() => store.tokenWarningLevel);
const tokenRemaining = computed(() => store.contextRemaining);
const tokenUsage = computed(() => store.tokenUsage);
const contextWindow = computed(() => store.contextWindow);

const isEmpty = computed(() => store.messages.length === 0);
const canSend = computed(() => inputText.value.trim().length > 0);

onMounted(() => {
  setupListeners();
  // Push current credentials to the Rust backend on mount
  store.configureAgent();
});

onUnmounted(() => {
  teardownListeners();
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
                <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
                  <rect x="2.5" y="2.5" width="9" height="9" rx="1.5" stroke="currentColor" stroke-width="1.5"/>
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
              <!-- Left: model name with hover dropdown -->
              <div class="footbar-left">
                <div
                  class="model-name-wrap"
                  @mouseenter="isModelDropdownVisible = true"
                  @mouseleave="isModelDropdownVisible = false"
                >
                  <span class="model-name-label">{{ configStore.activeModelName }}</span>
                  <svg width="10" height="10" viewBox="0 0 10 10" fill="none" class="model-chevron">
                    <path d="M2.5 3.5L5 6L7.5 3.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
                  </svg>
                  <!-- Model dropdown -->
                  <div v-if="isModelDropdownVisible" class="model-dropdown">
                    <div v-for="group in modelGroups" :key="group.label" class="model-dropdown-group">
                      <div class="model-dropdown-group-label">{{ group.label }}</div>
                      <div
                        v-for="m in group.models"
                        :key="m.value"
                        class="model-dropdown-item"
                        :class="{ active: m.value === configStore.activeModelId }"
                        @click="selectModel(m.value)"
                      >
                        <span class="model-dropdown-dot" :class="m.provider === 'deepseek' ? 'dot-deepseek' : 'dot-volc'"></span>
                        {{ m.label }}
                      </div>
                    </div>
                  </div>
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

/* Send circle — pill button pinned to the right */
.send-circle {
  width: 38px;
  height: 38px;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.05);
  color: #bbb;
  cursor: pointer;
  transition: all 0.25s cubic-bezier(0.25, 0.1, 0.25, 1);
  flex-shrink: 0;
}

.send-circle:hover {
  background: rgba(0, 0, 0, 0.08);
  color: #888;
}

.send-circle.active {
  background: #007aff;
  color: #fff;
  box-shadow: 0 2px 12px rgba(0, 122, 255, 0.3);
}

.send-circle.active:hover {
  background: #0062cc;
  transform: scale(1.05);
  box-shadow: 0 4px 16px rgba(0, 122, 255, 0.35);
}

.send-circle[disabled] {
  opacity: 0.3;
  cursor: default;
  transform: none !important;
}

.chat-panel.dark .send-circle {
  background: rgba(255, 255, 255, 0.08);
  color: #888;
}

.chat-panel.dark .send-circle:hover {
  background: rgba(255, 255, 255, 0.12);
  color: #ccc;
}

.chat-panel.dark .send-circle.active {
  background: #0a84ff;
  color: #fff;
  box-shadow: 0 2px 12px rgba(10, 132, 255, 0.35);
}

.chat-panel.dark .send-circle.active:hover {
  background: #409cff;
}

.chat-panel.dark .send-circle[disabled] {
  opacity: 0.2;
}

/* Stop circle */
.stop-circle {
  background: rgba(239, 68, 68, 0.08);
  color: #ef4444;
}

.stop-circle:hover {
  background: rgba(239, 68, 68, 0.15);
  color: #ef4444;
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

/* ── Left: model name with hover dropdown ───────── */

.footbar-left {
  display: flex;
  align-items: center;
  flex-shrink: 1;
}

.model-name-wrap {
  position: relative;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  cursor: pointer;
  user-select: none;
}

/* Invisible bridge to prevent mouse-out gap between trigger and dropdown */
.model-name-wrap::before {
  content: "";
  position: absolute;
  bottom: 100%;
  left: 0;
  right: 0;
  height: 14px;
  pointer-events: auto;
}

.model-name-label {
  font-size: 12.5px;
  font-weight: 500;
  color: #c8c8c8;
  white-space: nowrap;
  letter-spacing: 0.01em;
  transition: color 0.15s ease;
}

.model-name-wrap:hover .model-name-label {
  color: #e0e0e0;
}

.chat-panel.dark .model-name-label {
  color: #9ca3af;
}

.chat-panel.dark .model-name-wrap:hover .model-name-label {
  color: #d1d5db;
}

/* Chevron arrow */
.model-chevron {
  color: #6b7280;
  flex-shrink: 0;
  transition: transform 0.2s ease;
}

.model-name-wrap:hover .model-chevron {
  color: #9ca3af;
  transform: rotate(180deg);
}

/* ── Model dropdown ────────────────────────────── */

.model-dropdown {
  position: absolute;
  bottom: calc(100% + 8px);
  left: 0;
  min-width: 220px;
  max-height: 420px;
  overflow-y: auto;
  padding: 8px;
  background: rgba(255, 255, 255, 0.97);
  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 14px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.12), 0 4px 12px rgba(0, 0, 0, 0.06);
  z-index: 30;
  animation: dropdownIn 0.18s cubic-bezier(0.25, 0.1, 0.25, 1);
}

.chat-panel.dark .model-dropdown {
  background: rgba(36, 36, 42, 0.97);
  border-color: rgba(255, 255, 255, 0.08);
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
}

@keyframes dropdownIn {
  from {
    opacity: 0;
    transform: translateY(4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.model-dropdown-group-label {
  font-size: 10.5px;
  font-weight: 600;
  color: #9ca3af;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  padding: 8px 10px 4px;
}

.chat-panel.dark .model-dropdown-group-label {
  color: #6b7280;
}

.model-dropdown-group-label:first-child {
  padding-top: 4px;
}

.model-dropdown-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 10px;
  font-size: 13.5px;
  font-weight: 500;
  color: #333;
  cursor: pointer;
  transition: background 0.12s ease;
}

.model-dropdown-item:hover {
  background: rgba(0, 0, 0, 0.05);
}

.model-dropdown-item.active {
  background: rgba(99, 102, 241, 0.08);
  color: #6366f1;
}

.chat-panel.dark .model-dropdown-item {
  color: #ddd;
}

.chat-panel.dark .model-dropdown-item:hover {
  background: rgba(255, 255, 255, 0.06);
}

.chat-panel.dark .model-dropdown-item.active {
  background: rgba(129, 140, 248, 0.12);
  color: #a5b4fc;
}

/* Provider dot */
.model-dropdown-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
}

.dot-deepseek {
  background: #10b981;
}

.dot-volc {
  background: #6366f1;
}

/* ── Right: context capacity circle + auto-execute ── */
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
