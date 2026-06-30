<script setup lang="ts">
import { ref, nextTick, watch, onMounted, onUnmounted, onBeforeUpdate, computed } from "vue";
import { NInput, NScrollbar, NSelect, useMessage } from "naive-ui";
import MessageItem from "./MessageItem.vue";
import TodoPanel from "./TodoPanel.vue";
import PlanApprovalCard from "../approval/PlanApprovalCard.vue";
import { useAgentStore } from "../../stores/agent";
import { useThemeStore } from "../../stores/theme";
import { ALL_MODELS, useConfigStore } from "../../stores/config";
import { useProjectStore } from "../../stores/project";
import { useTauriEvent } from "../../composables/useTauriEvent";
import { useReviewStore } from "../../stores/review";

const store = useAgentStore();
const themeStore = useThemeStore();
const configStore = useConfigStore();
const msg = useMessage();
const { setupListeners, teardownListeners } = useTauriEvent();
const projectStore = useProjectStore();
const reviewStore = useReviewStore();
/** 当前项目名：用于空状态引导语，实时告知用户正在哪个项目下操作。 */
const currentProjectName = computed(() => projectStore.activeProject?.name ?? "Xuflow");
/** 空状态引导标题：动态替换项目名，让用户明确操作上下文。 */
const heroTitle = computed(() => `在 ${currentProjectName.value} 中开始你的 AI 编程...`);
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
const selectableModelIds = computed(() => ALL_MODELS.map((model) => model.value));

const activeStreamingMessageIndex = computed(() => {
  for (let i = store.messages.length - 1; i >= 0; i -= 1) {
    const msg = store.messages[i];
    if (msg.role === "assistant" && !msg.done) return i;
  }
  return -1;
});

let modelWheelLastAt = 0;
function handleModelWheel(e: WheelEvent) {
  if (Math.abs(e.deltaX) > Math.abs(e.deltaY)) return;

  e.preventDefault();
  e.stopPropagation();

  const now = Date.now();
  if (now - modelWheelLastAt < 120) return;
  modelWheelLastAt = now;

  const ids = selectableModelIds.value;
  if (ids.length === 0) return;

  const currentIndex = ids.indexOf(configStore.activeModelId);
  const fallbackIndex = currentIndex === -1 ? 0 : currentIndex;
  const direction = e.deltaY > 0 ? 1 : -1;
  const nextIndex = (fallbackIndex + direction + ids.length) % ids.length;
  configStore.activeModelId = ids[nextIndex];
}

// ── 滚动位置记忆：按会话 ID 记住用户停留的滚动位置 ──
// 切换会话时保存旧位置、恢复新位置；用户滚动时持续追踪当前位置。
// 仅在当前会话收到新消息时才自动滚到底部，切换会话时保持之前的位置。
const SCROLL_POSITIONS_STORAGE_KEY = "xuflow-conversation-scroll-top-v1";

function loadScrollPositions(): Map<string, number> {
  try {
    const raw = localStorage.getItem(SCROLL_POSITIONS_STORAGE_KEY);
    if (!raw) return new Map();
    const entries = JSON.parse(raw);
    if (!Array.isArray(entries)) return new Map();
    return new Map(
      entries.filter(
        (entry): entry is [string, number] =>
          Array.isArray(entry) &&
          typeof entry[0] === "string" &&
          typeof entry[1] === "number"
      )
    );
  } catch (e) {
    console.error("[chat] Failed to load scroll positions:", e);
    return new Map();
  }
}

function persistScrollPositions() {
  try {
    localStorage.setItem(
      SCROLL_POSITIONS_STORAGE_KEY,
      JSON.stringify([...scrollPositions.value.entries()])
    );
  } catch (e) {
    console.error("[chat] Failed to save scroll positions:", e);
  }
}

const scrollPositions = ref<Map<string, number>>(loadScrollPositions());
let restoringScrollFor: string | null = null;
let restoreScrollTimer: ReturnType<typeof setTimeout> | null = null;
let renderedConversationId: string | null = projectStore.activeConversationId;

/** 获取 NScrollbar 内部实际滚动的容器元素。 */
function getScrollElement(): HTMLElement | null {
  const container = scrollRef.value?.$el as HTMLElement | undefined;
  if (!container) return null;
  return (
    container.querySelector(".n-scrollbar-container") ??
    container.querySelector(".n-scrollbar-content") ??
    container
  ) as HTMLElement | null;
}

function getCurrentScrollTop(el = getScrollElement()): number | null {
  const scrollbar = scrollRef.value as any;
  if (typeof scrollbar?.containerScrollTop === "number") {
    return scrollbar.containerScrollTop;
  }
  if (!el) return null;
  return el.scrollTop;
}

function getMaxScrollTop(el = getScrollElement()): number {
  if (!el) return 0;
  return Math.max(0, el.scrollHeight - el.clientHeight);
}

function scrollChatTo(top: number) {
  scrollRef.value?.scrollTo({ top, behavior: "auto" });
  const el = getScrollElement();
  if (el) {
    el.scrollTop = top;
  }
}

/** 将指定会话的滚动位置保存到 Map 中，用于下次切回时恢复。 */
function saveCurrentScrollPosition(
  convId = projectStore.activeConversationId,
  el = getScrollElement()
) {
  if (!convId) return;
  if (restoringScrollFor === convId) return;
  const scrollTop = getCurrentScrollTop(el);
  if (scrollTop === null) return;
  scrollPositions.value.set(convId, scrollTop);
  persistScrollPositions();
}

function clearPendingScrollSave() {
  if (!scrollTrackTimer) return;
  clearTimeout(scrollTrackTimer);
  scrollTrackTimer = null;
}

function applyScrollPosition(convId: string, fallbackToBottom: boolean) {
  const el = getScrollElement();
  if (!el) return false;

  const savedTop = scrollPositions.value.get(convId);
  if (savedTop !== undefined) {
    const maxTop = getMaxScrollTop(el);
    scrollChatTo(Math.min(Math.max(0, savedTop), maxTop));
  } else if (fallbackToBottom) {
    scrollChatTo(el.scrollHeight);
  }

  return true;
}

function restoreConversationScroll(convId = projectStore.activeConversationId, fallbackToBottom = true) {
  if (!convId) return;
  if (restoreScrollTimer) {
    clearTimeout(restoreScrollTimer);
    restoreScrollTimer = null;
  }

  restoringScrollFor = convId;
  let attempt = 0;
  const delays = [0, 16, 32, 80, 160, 320];

  const run = () => {
    if (projectStore.activeConversationId !== convId) {
      if (restoringScrollFor === convId) restoringScrollFor = null;
      clearPendingScrollSave();
      return;
    }

    renderedConversationId = convId;
    clearPendingScrollSave();
    applyScrollPosition(convId, fallbackToBottom);

    attempt += 1;
    if (attempt < delays.length) {
      restoreScrollTimer = setTimeout(run, delays[attempt]);
      return;
    }

    restoreScrollTimer = null;
    clearPendingScrollSave();
    if (restoringScrollFor === convId) restoringScrollFor = null;
  };

  nextTick(run);
}

/** 持续追踪滚动位置（停止滚动后保存），保证记录的是用户真正停留的位置。 */
let scrollTrackTimer: ReturnType<typeof setTimeout> | null = null;

function scheduleScrollSave(convId: string | null, scrollTop: number | null) {
  // 没有活跃会话时不追踪滚动位置（例如 DOM 切换过渡期间）
  if (!convId) return;
  if (restoringScrollFor === convId) return;
  if (scrollTop === null) return;
  clearPendingScrollSave();
  scrollTrackTimer = setTimeout(() => {
    scrollTrackTimer = null;
    if (!convId) return;
    scrollPositions.value.set(convId, scrollTop);
    persistScrollPositions();
  }, 220);
}

function handleChatScroll(e: Event) {
  const target = e.target as HTMLElement | null;
  const top = typeof target?.scrollTop === "number" ? target.scrollTop : getCurrentScrollTop();
  scheduleScrollSave(renderedConversationId, top);
}

function persistCurrentScrollBeforeUnload() {
  saveCurrentScrollPosition(renderedConversationId, getScrollElement());
}

onMounted(() => {
  setupListeners();
  renderedConversationId = projectStore.activeConversationId;
  window.addEventListener("beforeunload", persistCurrentScrollBeforeUnload);
  window.addEventListener("pagehide", persistCurrentScrollBeforeUnload);
  // Push current credentials to the Rust backend on mount
  store.configureAgent();
  // Attach smooth scrolling once the scrollbar is rendered
  nextTick(() => {
    restoreConversationScroll();
  });
});

onBeforeUpdate(() => {
  const activeId = projectStore.activeConversationId;
  if (renderedConversationId && renderedConversationId !== activeId) {
    const currentScrollEl = getScrollElement();
    clearPendingScrollSave();
    saveCurrentScrollPosition(renderedConversationId, currentScrollEl);
    // 将会话追踪 ID 置空，防止 DOM 切换期间（消息内容替换导致
    // 滚动容器高度变化时）触发的 scroll 事件把位置错误写入旧会话。
    renderedConversationId = null;
  }
});

onUnmounted(() => {
  saveCurrentScrollPosition(renderedConversationId, getScrollElement());
  window.removeEventListener("beforeunload", persistCurrentScrollBeforeUnload);
  window.removeEventListener("pagehide", persistCurrentScrollBeforeUnload);
  teardownListeners();
  // Clean up pending scroll timers
  if (scrollTrackTimer) clearTimeout(scrollTrackTimer);
  if (restoreScrollTimer) clearTimeout(restoreScrollTimer);
});

// Re-attach smooth scroll handler when the scrollbar appears (first message)
watch(isEmpty, (empty) => {
  if (!empty) {
    nextTick(() => {
      restoreConversationScroll();
    });
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

  // 确保存在活跃会话；没有则按需创建（隐藏，等 AI 回复后自动提炼标题再显示）
  const projectStore = useProjectStore();
  if (!projectStore.activeConversation) {
    if (!projectStore.activeProject) {
      const project = projectStore.createProject("默认项目");
      projectStore.switchTo(project.id);
    }
    if (projectStore.activeProject && !projectStore.activeConversation) {
      // 创建隐藏会话，不立即显示在侧边栏；AI 回复后由 agent:done 自动提炼标题并显示
      const conv = projectStore.createConversation(
        projectStore.activeProject.id,
        undefined,
        undefined,
        false,
      );
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
      scrollChatTo(getScrollElement()?.scrollHeight ?? 99999);
    });
  }
}

function handleStop() {
  store.stopGeneration();
}

// ── 会话切换时保存/恢复滚动位置 ──
// 切走时记住当前位置，切回时恢复；没有历史位置则默认滚到底部。
watch(
  () => projectStore.activeConversationId,
  (newId, oldId) => {
    if (oldId) {
      const oldScrollEl = getScrollElement();
      clearPendingScrollSave();
      saveCurrentScrollPosition(oldId, oldScrollEl);
    }
    if (newId) {
      restoreConversationScroll(newId);
    } else {
      renderedConversationId = null;
    }
  },
  { flush: "pre", immediate: true }
);

// 仅在当前会话收到新消息时自动滚到底部（排除切换会话导致的 length 变化）
watch(
  () => [projectStore.activeConversationId, store.messages.length] as const,
  ([convId, length], [oldConvId, oldLength]) => {
    if (!convId) return;
    if (convId !== oldConvId) return;
    if (restoringScrollFor === convId) return;
    if (length <= oldLength) return;
    if (!store.isRunning && !sending.value) {
      restoreConversationScroll(convId, false);
      return;
    }
    nextTick(() => {
      scrollChatTo(getScrollElement()?.scrollHeight ?? 99999);
    });
  },
  { flush: "post" }
);
</script>

<template>
  <div class="chat-panel" :class="{ dark: themeStore.isDark, 'is-empty': isEmpty }">
    <!-- 对话区域右上角：侧边栏显示/隐藏切换按钮，用于快速打开代码审查面板 -->
    <button
      class="sidebar-toggle-btn"
      :class="{ active: reviewStore.visible }"
      @click="reviewStore.togglePanel()"
      :title="reviewStore.visible ? '隐藏审查面板' : '显示审查面板'"
    >
      <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
        <!-- 左侧面板线条 -->
        <rect x="2" y="3" width="5" height="12" rx="1" stroke="currentColor" stroke-width="1.4" />
        <!-- 分隔线 -->
        <line x1="9" y1="4" x2="9" y2="14" stroke="currentColor" stroke-width="1.2" opacity="0.4" />
        <!-- 右侧内容区域 -->
        <rect x="11" y="5" width="5" height="4" rx="0.8" stroke="currentColor" stroke-width="1.2" />
        <rect x="11" y="11" width="3" height="2" rx="0.6" stroke="currentColor" stroke-width="1" opacity="0.5" />
      </svg>
    </button>

    <!-- 空状态引导页：居中展示核心引导提示语，移除 Logo 与说明文字，将视觉焦点集中在引导词与输入框上 -->
    <!-- 通过大字号、适中字重的 Hero 标题营造类似 Codex/Cursor 的起始引导感，降低用户上手门槛 -->
    <div v-if="isEmpty" class="welcome-container">
      <div class="welcome-content">
        <h1 class="hero-title">{{ heroTitle }}</h1>
      </div>
    </div>

    <!-- Chat body: keep the footer overlay full width while sharing one centered content width -->
    <div v-else class="chat-body">
      <NScrollbar
        ref="scrollRef"
        class="chat-scroll"
        @scroll="handleChatScroll"
      >
        <div class="chat-content-shell max-w-4xl mx-auto">
          <div class="message-list">
            <!-- Structured overlays: plan approval + todo list -->
            <PlanApprovalCard />
            <TodoPanel />
            <MessageItem
              v-for="(msg, i) in store.messages"
              :key="i"
              :message="msg"
              :active-streaming="store.isRunning && i === activeStreamingMessageIndex"
            />
          </div>
        </div>
      </NScrollbar>
    </div> <!-- /chat-body -->

    <!-- 底部输入区域：胶囊形输入卡片 + 独立功能栏，始终可见 -->
    <!-- 将功能栏从输入卡片内部提取到外部，形成"输入框在上、配置栏在下"的独立功能组，空状态时由父级 justify-content:center 居中 -->
    <div class="chat-footer-outer">
      <div class="chat-footer-shell">
        <!-- 胶囊形输入卡片：大圆角、微凸起背景 -->
        <div class="chat-footer">
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
          </div>
        <!-- 底部功能栏：模型选择、上下文容量、自动执行/先规划开关，独立于输入卡片 -->
        <div class="chat-footbar">
              <!-- Left: model selector — plain text label over transparent NSelect -->
              <div class="footbar-left">
                <div class="model-select-wrap" @wheel.capture="handleModelWheel">
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
</template>

<style scoped>
.chat-panel {
  position: relative;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: #fafafa;
  transition: background-color 0.3s ease;
}

.chat-panel.dark {
  background: #1a1a20;
}

/* 对话区域右上角侧边栏切换按钮：悬浮于内容之上，与消息区保持距离 */
.sidebar-toggle-btn {
  position: absolute;
  top: 10px;
  right: 14px;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: #9ca3af;
  cursor: pointer;
  transition: all 0.2s ease;
  z-index: 10;
}

.sidebar-toggle-btn:hover {
  background: rgba(0, 0, 0, 0.05);
  color: #374151;
}

.sidebar-toggle-btn.active {
  color: #6366f1;
  background: rgba(99, 102, 241, 0.08);
}

.sidebar-toggle-btn.active:hover {
  color: #4f46e5;
  background: rgba(99, 102, 241, 0.14);
}

.chat-panel.dark .sidebar-toggle-btn {
  color: #6b7280;
}

.chat-panel.dark .sidebar-toggle-btn:hover {
  background: rgba(255, 255, 255, 0.06);
  color: #d1d5db;
}

.chat-panel.dark .sidebar-toggle-btn.active {
  color: #818cf8;
  background: rgba(129, 140, 248, 0.12);
}

.chat-panel.dark .sidebar-toggle-btn.active:hover {
  color: #a5b4fc;
  background: rgba(165, 180, 252, 0.18);
}

/* 空状态时整体内容垂直居中（引导语 + 输入框作为一组），上下留出呼吸空间 */
.chat-panel.is-empty {
  justify-content: center;
  gap: 36px;
}

/* ── 空状态引导页：垂直居中，为 Hero 标题与下方输入框提供呼吸空间 ── */
/* 不使用 flex:1，让 welcome + footer 作为整体由父级 justify-content:center 居中 */
.welcome-container {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 20px;
}

.welcome-content {
  max-width: 700px;
  width: 100%;
  text-align: center;
}

/* 核心引导提示语：大字号（26px）、适中字重（550），营造 Codex/Cursor 起始引导感 */
.hero-title {
  font-family: "Inter", "SF Pro Display", "SF Pro Text", -apple-system, BlinkMacSystemFont, "Helvetica Neue", sans-serif;
  font-size: 26px;
  font-weight: 550;
  letter-spacing: -0.02em;
  line-height: 1.35;
  margin: 0;
  color: #1c1c1c;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

.chat-panel.dark .hero-title {
  color: #e8e8ed;
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

/* ── 底部输入区外层：非空状态底部吸附 + 渐变遮罩；空状态由父级居中控制 ── */
/* 渐变仅在非空状态下生效，模拟消息内容淡入输入框的视觉效果 */
.chat-footer-outer {
  flex-shrink: 0;
  width: 100%;
  display: flex;
  justify-content: center;
  padding: 0 24px 24px;
}

.chat-footer-shell {
  width: 100%;
  max-width: 700px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
}

/* 非空状态：消息区底部渐变遮罩 */
.chat-panel:not(.is-empty) .chat-footer-outer {
  padding: 14px 24px 24px;
  background: linear-gradient(to top, rgba(250, 250, 250, 0.92) 0%, rgba(250, 250, 250, 0) 60%);
}

.chat-panel.dark:not(.is-empty) .chat-footer-outer {
  background: linear-gradient(to top, rgba(26, 26, 32, 0.94) 0%, rgba(26, 26, 32, 0) 60%);
}

/* ── 胶囊形输入卡片：大圆角（28px）、微凸起、背景比主背景稍亮一阶 ── */
/* 暗色模式下使用 #2C2C2E 营造层次感，亮色模式使用纯白 + 细边框 */
.chat-footer {
  position: relative;
  width: 100%;
  padding: 10px 8px 10px 12px;
  background: #ffffff;
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 14px;
  box-shadow:
    0 2px 8px rgba(0, 0, 0, 0.04),
    0 0 0 1px rgba(0, 0, 0, 0.02);
  transition: background 0.4s ease, border-color 0.4s ease, box-shadow 0.4s ease;
  font-family: -apple-system, BlinkMacSystemFont, "SF Pro Display", "SF Pro Text", "Helvetica Neue", sans-serif;
}

.chat-panel.dark .chat-footer {
  background: #2C2C2E;
  border-color: rgba(255, 255, 255, 0.08);
  box-shadow:
    0 2px 12px rgba(0, 0, 0, 0.3),
    0 0 0 1px rgba(255, 255, 255, 0.04);
}

/* ── 输入区域：flex:1 占据胶囊内剩余空间，内部 NInput 透明融合 ── */
.input-area {
  position: relative;
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
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
  font-size: 15px;
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

/* ── 输入行：输入框 + 发送按钮水平并排，胶囊内垂直居中 ── */
.input-row {
  display: flex;
  align-items: center;
  gap: 8px;
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

/* ── 底部功能栏：独立一行，小字体，与上方胶囊形成完整功能组 ── */
/* 从输入卡片内部提取到外部，去掉背景/边框/圆角，呈现轻量纯文字工具栏 */
.chat-footbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding: 0 10px;
  height: 28px;
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
  color: #6b7280;
  white-space: nowrap;
  letter-spacing: 0.01em;
  transition: color 0.15s ease, opacity 0.15s ease, transform 0.15s ease;
  pointer-events: none;
}

.chat-panel.dark .model-name-label {
  color: #d4d4d4;
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
  color: #374151;
}

.chat-panel.dark .model-select-wrap:hover .model-name-label {
  color: #f0f0f0;
}

/* Chevron */
.model-chevron {
  color: #9ca3af;
  flex-shrink: 0;
  pointer-events: none;
  transition: transform 0.2s ease, color 0.15s ease;
}

.model-select-wrap:hover .model-chevron {
  color: #6b7280;
  transform: rotate(180deg);
}

.chat-panel.dark .model-chevron {
  color: #6b7280;
}

.chat-panel.dark .model-select-wrap:hover .model-chevron {
  color: #9ca3af;
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

/* ── 模型选择下拉菜单：视觉样式统一由 App.vue 全局样式控制（Naive UI Teleport 到 body，scoped 穿透无效）── */
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
  color: #6b7280;
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
    max-width: 100%;
  }

  .chat-footer {
    border-radius: 12px;
    padding: 4px 6px 4px 14px;
  }

  .chat-footbar {
    height: auto;
    min-height: 28px;
    gap: 8px;
    padding: 6px 8px;
    flex-wrap: wrap;
  }

  .footbar-right {
    margin-left: auto;
  }

  .hero-title {
    font-size: 22px;
  }

  .welcome-container {
    padding: 0 16px;
  }
}
</style>
