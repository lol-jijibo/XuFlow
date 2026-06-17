<script setup lang="ts">
import { ref, nextTick, watch, onMounted, onUnmounted, computed } from "vue";
import { NInput, NScrollbar } from "naive-ui";
import MessageItem from "./MessageItem.vue";
import { useAgentStore } from "../../stores/agent";
import { useThemeStore } from "../../stores/theme";
import { useConfigStore } from "../../stores/config";
import { useProjectStore } from "../../stores/project";
import { useTauriEvent } from "../../composables/useTauriEvent";

const store = useAgentStore();
const themeStore = useThemeStore();
const configStore = useConfigStore();
const { setupListeners, teardownListeners } = useTauriEvent();
const inputText = ref("");
const scrollRef = ref<InstanceType<typeof NScrollbar> | null>(null);
const sending = ref(false);
const activeExt = ref<"model" | "memory" | "tools" | null>(null);

// ── Footbar state ──
const isInputFocused = ref(false);
const isFooterHovered = ref(false);
const isFootbarHovered = ref(false);
const isInfoTooltipVisible = ref(false);
const autoExecute = ref(false);

// Token usage (placeholder — wired to real Usage data later)
const tokenUsed = ref(12000);   // 12k tokens
const tokenMax = ref(128000);   // 128k tokens
const tokenPercent = computed(() => {
  if (tokenMax.value <= 0) return 0;
  return Math.min(100, Math.round((tokenUsed.value / tokenMax.value) * 100));
});
const tokenUsedLabel = computed(() => {
  return tokenUsed.value >= 1000 ? `${(tokenUsed.value / 1000).toFixed(0)}k` : `${tokenUsed.value}`;
});
const tokenMaxLabel = computed(() => {
  return tokenMax.value >= 1000 ? `${(tokenMax.value / 1000).toFixed(0)}k` : `${tokenMax.value}`;
});

// Footbar opacity: dimmed when not interacting, full when focused/hovered
const footbarOpacity = computed(() => {
  return isInputFocused.value || isFooterHovered.value || isFootbarHovered.value ? 1 : 0.5;
});

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
            <MessageItem
              v-for="(msg, i) in store.messages"
              :key="i"
              :message="msg"
            />
          </div>
        </div>
      </NScrollbar>

      <!-- Footer: full-width overlay + centered input card -->
      <div class="chat-footer-outer">
        <div class="chat-footer-shell chat-content-shell max-w-4xl mx-auto">
          <div
            class="chat-footer"
            @mouseenter="isFooterHovered = true"
            @mouseleave="isFooterHovered = false"
          >
            <!-- Text input — white canvas feel -->
            <div class="input-area">
              <NInput
                :value="inputText"
                @update:value="inputText = $event"
                type="textarea"
                placeholder="向 Agent 下达指令..."
                :autosize="{ minRows: 1, maxRows: 6 }"
                :disabled="store.isRunning"
                @keydown.enter.exact.prevent="sendMessage"
                @focus="isInputFocused = true"
                @blur="isInputFocused = false"
                class="chat-input"
              />
            </div>

            <!-- Actions: extension pills + send -->
            <div class="actions-row">
              <div class="ext-pills">
                <button
                  class="ext-pill"
                  :class="{ active: activeExt === 'model' }"
                  @click="activeExt = activeExt === 'model' ? null : 'model'"
                >
                  🤖 模型
                </button>
                <button
                  class="ext-pill"
                  :class="{ active: activeExt === 'memory' }"
                  @click="activeExt = activeExt === 'memory' ? null : 'memory'"
                >
                  🧠 记忆
                </button>
                <button
                  class="ext-pill"
                  :class="{ active: activeExt === 'tools' }"
                  @click="activeExt = activeExt === 'tools' ? null : 'tools'"
                >
                  🔗 工具
                </button>
              </div>
              <div class="actions-right">
                <!-- Stop -->
                <button
                  v-if="store.isRunning"
                  class="send-circle stop-circle"
                  @click="handleStop"
                  title="停止生成"
                >
                  <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                    <rect x="2" y="2" width="8" height="8" rx="1.5" stroke="currentColor" stroke-width="1.5"/>
                  </svg>
                </button>
                <!-- Send arrow -->
                <button
                  v-else
                  class="send-circle"
                  :class="{ active: canSend && !sending }"
                  :disabled="!canSend || sending"
                  @click="sendMessage"
                  title="发送"
                >
                  <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
                    <path d="M9 14V4M5 8l4-4 4 4" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/>
                  </svg>
                </button>
              </div>
            </div>

            <!-- Floating extension card -->
            <div v-if="activeExt" class="ext-card">
              <template v-if="activeExt === 'model'">
                <div class="ext-card-item">
                  <span class="ext-dot" style="background:#6366f1"></span>
                  DeepSeek-V4-pro
                </div>
                <div class="ext-card-item">
                  <span class="ext-dot" style="background:#22c55e"></span>
                  GPT-4o
                </div>
                <div class="ext-card-item">
                  <span class="ext-dot" style="background:#f59e0b"></span>
                  Claude 3.5
                </div>
              </template>
              <template v-else-if="activeExt === 'memory'">
                <div class="ext-card-item">
                  <span class="ext-dot" style="background:#6366f1"></span>
                  短期记忆 · 开启
                </div>
                <div class="ext-card-item">
                  <span class="ext-dot" style="background:#6b7280"></span>
                  长期记忆 · 关闭
                </div>
              </template>
              <template v-else>
                <div class="ext-card-item">
                  <span class="ext-dot" style="background:#22c55e"></span>
                  文件搜索
                </div>
                <div class="ext-card-item">
                  <span class="ext-dot" style="background:#3b82f6"></span>
                  代码执行
                </div>
                <div class="ext-card-item">
                  <span class="ext-dot" style="background:#f59e0b"></span>
                  Web 搜索
                </div>
              </template>
            </div>

            <!-- ── Footbar: status strip attached below input ── -->
            <div
              class="chat-footbar"
              :style="{ opacity: footbarOpacity }"
              @mouseenter="isFootbarHovered = true"
              @mouseleave="isFootbarHovered = false"
            >
              <!-- Left: context capacity -->
              <div class="footbar-left">
                <!-- Token progress bar -->
                <div class="token-bar-wrapper">
                  <div class="token-bar-track">
                    <div
                      class="token-bar-fill"
                      :style="{ width: tokenPercent + '%' }"
                      :class="{ warn: tokenPercent > 75, danger: tokenPercent > 90 }"
                    ></div>
                  </div>
                </div>
                <span class="token-label">
                  上下文容量 ({{ tokenUsedLabel }} / {{ tokenMaxLabel }} tokens)
                </span>
                <!-- Info icon + tooltip -->
                <div
                  class="info-icon-wrap"
                  @mouseenter="isInfoTooltipVisible = true"
                  @mouseleave="isInfoTooltipVisible = false"
                >
                  <svg width="13" height="13" viewBox="0 0 13 13" fill="none" class="info-icon-svg">
                    <circle cx="6.5" cy="6.5" r="5.5" stroke="currentColor" stroke-width="1.1"/>
                    <path d="M6.5 5.8v3.5M6.5 3.8v.01" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
                  </svg>
                  <!-- Hover tooltip card -->
                  <div v-if="isInfoTooltipVisible" class="info-tooltip">
                    <p class="tooltip-title">上下文占用</p>
                    <p class="tooltip-desc">
                      当前已使用 <strong>{{ tokenUsedLabel }}</strong> / <strong>{{ tokenMaxLabel }}</strong> tokens（{{ tokenPercent }}%）。
                      包含系统提示、对话历史和知识库引用。
                    </p>
                  </div>
                </div>
              </div>

              <!-- Right: status + toggles -->
              <div class="footbar-right">
                <!-- Auto-execute toggle -->
                <button
                  class="ios-toggle"
                  :class="{ active: autoExecute }"
                  @click="autoExecute = !autoExecute"
                  title="自动执行工具调用"
                >
                  <span class="ios-toggle-knob"></span>
                </button>
                <span class="toggle-label">自动执行</span>

                <!-- Divider -->
                <span class="footbar-divider"></span>

                <!-- Status indicator -->
                <span class="status-dot" :class="{ running: store.isRunning }"></span>
                <span class="status-text">{{ store.isRunning ? 'Generating' : 'Ready' }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div> <!-- /chat-body -->
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
  padding: 40px 40px 48px;
}

/* ── Footer overlay: spans the panel, while the input card stays centered ── */
.chat-footer-outer {
  flex-shrink: 0;
  width: 100%;
  padding: 12px 20px 20px;
  background: linear-gradient(to top, rgba(250, 250, 250, 0.92) 0%, rgba(250, 250, 250, 0) 60%);
}

.chat-panel.dark .chat-footer-outer {
  background: linear-gradient(to top, rgba(26, 26, 32, 0.94) 0%, rgba(26, 26, 32, 0) 60%);
}

.chat-footer-shell {
  padding: 0 40px;
}

/* ── Apple-style frosted glass input + footbar ── */
.chat-footer {
  position: relative;
  padding: 16px 20px 0;
  width: 100%;
  background: rgba(255, 255, 255, 0.72);
  backdrop-filter: blur(24px) saturate(1.2);
  -webkit-backdrop-filter: blur(24px) saturate(1.2);
  border: 1px solid rgba(255, 255, 255, 0.3);
  border-radius: 24px;
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
}

.input-area :deep(.n-input__textarea-el) {
  resize: none;
  font-size: 16px;
  line-height: 1.6;
  font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Helvetica Neue", sans-serif;
  background: transparent !important;
  color: #1a1a1a;
  padding: 4px 0;
  letter-spacing: -0.01em;
}

.chat-panel.dark .input-area :deep(.n-input__textarea-el) {
  color: #ececf0;
}

.chat-panel.dark .input-area :deep(.n-input__placeholder) {
  color: #888 !important;
}

/* ── Actions row ── */
.actions-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 10px;
}

/* Extension pills */
.ext-pills {
  display: flex;
  align-items: center;
  gap: 6px;
}

.ext-pill {
  font-size: 12px;
  font-weight: 500;
  color: #999;
  background: rgba(0, 0, 0, 0.03);
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 20px;
  padding: 4px 12px;
  cursor: pointer;
  user-select: none;
  transition: all 0.2s ease;
}

.ext-pill:hover {
  background: rgba(0, 0, 0, 0.06);
  color: #555;
  border-color: rgba(0, 0, 0, 0.1);
}

.ext-pill.active {
  background: rgba(99, 102, 241, 0.1);
  border-color: rgba(99, 102, 241, 0.2);
  color: #6366f1;
}

.chat-panel.dark .ext-pill {
  color: #999;
  background: rgba(255, 255, 255, 0.04);
  border-color: rgba(255, 255, 255, 0.06);
}

.chat-panel.dark .ext-pill:hover {
  background: rgba(255, 255, 255, 0.08);
  color: #ddd;
  border-color: rgba(255, 255, 255, 0.12);
}

.chat-panel.dark .ext-pill.active {
  background: rgba(129, 140, 248, 0.15);
  border-color: rgba(129, 140, 248, 0.25);
  color: #a5b4fc;
}

/* Actions right */
.actions-right {
  display: flex;
  align-items: center;
  gap: 6px;
}

/* Send circle — Apple-style pill button */
.send-circle {
  width: 34px;
  height: 34px;
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

/* ── Floating extension card ── */
.ext-card {
  margin-top: 10px;
  padding: 8px;
  background: rgba(255, 255, 255, 0.88);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 16px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.06);
  animation: extCardIn 0.2s cubic-bezier(0.25, 0.1, 0.25, 1);
}

.chat-panel.dark .ext-card {
  background: rgba(36, 36, 42, 0.92);
  border-color: rgba(255, 255, 255, 0.06);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
}

@keyframes extCardIn {
  from {
    opacity: 0;
    transform: translateY(-4px) scale(0.97);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}

.ext-card-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-radius: 12px;
  font-size: 14px;
  font-weight: 500;
  color: #333;
  cursor: pointer;
  transition: background 0.15s ease;
}

.ext-card-item:hover {
  background: rgba(0, 0, 0, 0.04);
}

.chat-panel.dark .ext-card-item {
  color: #ddd;
}

.chat-panel.dark .ext-card-item:hover {
  background: rgba(255, 255, 255, 0.05);
}

.ext-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

/* ── Footbar ─────────────────────────────────────── */

.chat-footbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 42px;
  padding: 0 4px;
  margin: 10px -20px 0;          /* full-width bleed + top gap from actions */
  border-top: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 0 0 24px 24px;   /* match parent bottom corners */
  background: rgba(0, 0, 0, 0.4);
  backdrop-filter: blur(4px);
  -webkit-backdrop-filter: blur(4px);
  transition: opacity 0.3s cubic-bezier(0.25, 0.1, 0.25, 1);
  user-select: none;
}

/* ── Left: context capacity ─────────────────────── */

.footbar-left {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  margin-left: 4px;
}

/* Token progress bar — thin, minimal */
.token-bar-wrapper {
  flex-shrink: 0;
}

.token-bar-track {
  width: 56px;
  height: 3px;
  border-radius: 2px;
  background: rgba(0, 0, 0, 0.08);
  overflow: hidden;
}

.chat-panel.dark .token-bar-track {
  background: rgba(255, 255, 255, 0.1);
}

.token-bar-fill {
  height: 100%;
  border-radius: 2px;
  background: #6366f1;
  transition: width 0.5s cubic-bezier(0.25, 0.1, 0.25, 1), background 0.3s ease;
}

.token-bar-fill.warn {
  background: #f59e0b;
}

.token-bar-fill.danger {
  background: #ef4444;
}

/* Token text label */
.token-label {
  font-size: 11px;
  font-weight: 450;
  color: #999;
  white-space: nowrap;
  letter-spacing: 0.01em;
}

.chat-panel.dark .token-label {
  color: #888;
}

/* Info icon */
.info-icon-wrap {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  cursor: help;
}

.info-icon-svg {
  color: #bbb;
  transition: color 0.15s ease;
}

.info-icon-wrap:hover .info-icon-svg {
  color: #888;
}

.chat-panel.dark .info-icon-svg {
  color: #666;
}

.chat-panel.dark .info-icon-wrap:hover .info-icon-svg {
  color: #aaa;
}

/* Tooltip card */
.info-tooltip {
  position: absolute;
  bottom: calc(100% + 8px);
  left: 50%;
  transform: translateX(-50%);
  width: 220px;
  padding: 10px 14px;
  background: rgba(255, 255, 255, 0.96);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 12px;
  box-shadow: 0 8px 28px rgba(0, 0, 0, 0.1), 0 2px 8px rgba(0, 0, 0, 0.04);
  z-index: 20;
  animation: tooltipIn 0.18s cubic-bezier(0.25, 0.1, 0.25, 1);
}

.chat-panel.dark .info-tooltip {
  background: rgba(36, 36, 42, 0.96);
  border-color: rgba(255, 255, 255, 0.1);
  box-shadow: 0 8px 28px rgba(0, 0, 0, 0.4);
}

@keyframes tooltipIn {
  from {
    opacity: 0;
    transform: translateX(-50%) translateY(4px);
  }
  to {
    opacity: 1;
    transform: translateX(-50%) translateY(0);
  }
}

.tooltip-title {
  margin: 0 0 4px;
  font-size: 12px;
  font-weight: 600;
  color: #333;
  letter-spacing: 0.01em;
}

.chat-panel.dark .tooltip-title {
  color: #eee;
}

.tooltip-desc {
  margin: 0;
  font-size: 11px;
  line-height: 1.5;
  color: #888;
}

.chat-panel.dark .tooltip-desc {
  color: #999;
}

.tooltip-desc strong {
  color: #555;
  font-weight: 600;
}

.chat-panel.dark .tooltip-desc strong {
  color: #ccc;
}

/* ── Right: status + toggles ────────────────────── */

.footbar-right {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-right: 4px;
}

/* iOS-style mini toggle switch */
.ios-toggle {
  position: relative;
  width: 30px;
  height: 18px;
  padding: 0;
  border: none;
  border-radius: 9px;
  background: rgba(0, 0, 0, 0.15);
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
  width: 14px;
  height: 14px;
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
  font-size: 11px;
  font-weight: 450;
  color: #999;
  white-space: nowrap;
  letter-spacing: 0.01em;
}

.chat-panel.dark .toggle-label {
  color: #888;
}

/* Subtle vertical divider */
.footbar-divider {
  width: 1px;
  height: 14px;
  background: rgba(0, 0, 0, 0.08);
  margin: 0 4px;
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
  font-size: 11px;
  font-weight: 450;
  color: #999;
  white-space: nowrap;
  letter-spacing: 0.01em;
}

.chat-panel.dark .status-text {
  color: #888;
}

@media (max-width: 768px) {
  .message-list {
    padding: 28px 20px 36px;
  }

  .chat-footer-outer {
    padding: 10px 12px 14px;
  }

  .chat-footer-shell {
    padding: 0 8px;
  }
}
</style>
