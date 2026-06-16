<script setup lang="ts">
import { ref, nextTick, watch, onMounted, onUnmounted, computed } from "vue";
import { NInput, NButton, NScrollbar } from "naive-ui";
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
// 'auto' = 完全自动执行无需确认, 'approval' = 操作需用户手动批准
const permissionMode = ref<"auto" | "approval">("approval");

const isEmpty = computed(() => store.messages.length === 0);

function togglePermissionMode() {
  permissionMode.value = permissionMode.value === "approval" ? "auto" : "approval";
}

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

function insertPrompt(text: string) {
  inputText.value = text;
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
        <div class="welcome-prompts">
          <div
            class="prompt-card"
            @click="insertPrompt('帮我分析这个项目的架构')"
          >
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
              <path
                d="M3 4h14M3 10h10M3 16h6"
                stroke="currentColor"
                stroke-width="1.6"
                stroke-linecap="round"
              />
            </svg>
            <span>分析项目架构</span>
          </div>
          <div
            class="prompt-card"
            @click="insertPrompt('帮我找出代码中的 bug')"
          >
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
              <path
                d="M10 2v4M4.93 4.93l2.83 2.83M2 10h4M4.93 15.07l2.83-2.83M10 14v4M15.07 15.07l-2.83-2.83M18 10h-4M15.07 4.93l-2.83 2.83"
                stroke="currentColor"
                stroke-width="1.6"
                stroke-linecap="round"
              />
            </svg>
            <span>查找代码 Bug</span>
          </div>
          <div
            class="prompt-card"
            @click="insertPrompt('帮我优化这段代码的性能')"
          >
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
              <path
                d="M10 18V6M6 10l4-4 4 4"
                stroke="currentColor"
                stroke-width="1.6"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <path
                d="M3 3h14"
                stroke="currentColor"
                stroke-width="1.6"
                stroke-linecap="round"
              />
            </svg>
            <span>优化代码性能</span>
          </div>
          <div
            class="prompt-card"
            @click="insertPrompt('帮我写一个新功能：')"
          >
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
              <path
                d="M10 3v14M3 10h14"
                stroke="currentColor"
                stroke-width="1.6"
                stroke-linecap="round"
              />
            </svg>
            <span>编写新功能</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Messages -->
    <NScrollbar v-else ref="scrollRef" class="chat-scroll">
      <div class="message-list">
        <MessageItem
          v-for="(msg, i) in store.messages"
          :key="i"
          :message="msg"
        />
      </div>
    </NScrollbar>

    <!-- Input area -->
    <div class="chat-footer">
      <div class="input-area">
        <NInput
          v-model="inputText"
          type="textarea"
          placeholder="输入消息... (Enter 发送, Shift+Enter 换行)"
          :autosize="{ minRows: 2, maxRows: 8 }"
          :disabled="store.isRunning"
          @keydown.enter.exact.prevent="sendMessage"
          class="chat-input"
        />
      </div>
      <div class="input-toolbar">
        <div class="toolbar-left">
          <!-- + 添加文件按钮 -->
          <NButton quaternary circle size="small" title="添加文件" class="toolbar-btn">
            <template #icon>
              <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
                <path d="M9 3v12M3 9h12" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>
              </svg>
            </template>
          </NButton>
          <!-- 权限模式切换 -->
          <NButton
            quaternary
            size="small"
            class="toolbar-btn permission-btn"
            :class="{ auto: permissionMode === 'auto' }"
            @click="togglePermissionMode"
            :title="permissionMode === 'auto' ? '自动执行模式' : '请求授权模式'"
          >
            <template #icon>
              <!-- auto: 闪电图标 -->
              <svg v-if="permissionMode === 'auto'" width="16" height="16" viewBox="0 0 16 16" fill="none">
                <path d="M8.67 2L4 9h3.33L7.33 14 12 7H8.67L8.67 2z" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round"/>
              </svg>
              <!-- approval: 盾牌图标 -->
              <svg v-else width="16" height="16" viewBox="0 0 16 16" fill="none">
                <path d="M8 2l5 2.5v4c0 3.5-2.5 5.5-5 6.5-2.5-1-5-3-5-6.5v-4L8 2z" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round"/>
              </svg>
            </template>
          </NButton>
        </div>
        <div class="toolbar-right">
          <span class="model-label">{{ configStore.activeModelName }}</span>
          <!-- 运行中显示停止按钮 -->
          <NButton
            v-if="store.isRunning"
            type="error"
            size="small"
            @click="handleStop"
            class="send-btn"
            title="停止生成"
          >
            <template #icon>
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <rect x="3" y="3" width="10" height="10" rx="2" fill="currentColor"/>
              </svg>
            </template>
          </NButton>
          <!-- 发送按钮 -->
          <NButton
            v-else
            type="primary"
            size="small"
            :loading="sending"
            :disabled="!inputText.trim()"
            @click="sendMessage"
            class="send-btn"
            title="发送"
          >
            <template #icon>
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <path d="M4 8h8M10 5l3 3-3 3" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </template>
          </NButton>
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
  background: #f8fafc;
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
  color: #1e293b;
}

.chat-panel.dark .welcome-title {
  color: #e2e8f0;
}

.welcome-subtitle {
  font-size: 15px;
  color: #64748b;
  margin: 0 0 32px;
  line-height: 1.6;
}

.chat-panel.dark .welcome-subtitle {
  color: #94a3b8;
}

.welcome-prompts {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 12px;
}

.prompt-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 16px;
  background: #f8fafc;
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.2s ease;
  text-align: left;
  font-size: 14px;
  color: #475569;
}

.prompt-card:hover {
  background: #f1f5f9;
  border-color: #9ca3af;
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
}

.chat-panel.dark .prompt-card {
  background: #1a1a20;
  border-color: rgba(255, 255, 255, 0.08);
  color: #94a3b8;
}

.chat-panel.dark .prompt-card:hover {
  background: #24242d;
  border-color: #6b7280;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}

.prompt-card svg {
  flex-shrink: 0;
  color: #6b7280;
}

/* Messages */
.chat-scroll {
  flex: 1;
}

.message-list {
  padding: 32px;
  max-width: 1080px;
  margin: 0 auto;
}

/* ── Input area (single layer, no nesting) ── */
.chat-footer {
  padding: 12px 24px 16px;
  max-width: 1080px;
  width: 100%;
  margin: 0 auto;
}

/* 上行：文本输入 */
.input-area {
  margin-bottom: 8px;
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
  font-size: 14px;
  line-height: 1.6;
  background: transparent !important;
  color: #1e293b;
}

.chat-panel.dark .input-area :deep(.n-input__textarea-el) {
  color: #e2e8f0;
}

.chat-panel.dark .input-area :deep(.n-input__placeholder) {
  color: #64748b !important;
}

/* 下行：工具栏 */
.input-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.toolbar-left,
.toolbar-right {
  display: flex;
  align-items: center;
  gap: 6px;
}

.toolbar-btn {
  color: #94a3b8;
}

.toolbar-btn:hover {
  color: #475569;
}

.chat-panel.dark .toolbar-btn {
  color: #64748b;
}

.chat-panel.dark .toolbar-btn:hover {
  color: #9ca3af;
}

/* 权限模式按钮 */
.permission-btn.auto {
  color: #22c55e;
}

.permission-btn.auto:hover {
  color: #16a34a;
}

/* 模型名标签 */
.model-label {
  font-size: 11px;
  color: #94a3b8;
  font-family: "SF Mono", "Fira Code", monospace;
  margin-right: 4px;
  user-select: none;
}

.chat-panel.dark .model-label {
  color: #64748b;
}

/* 发送 / 停止按钮 */
.send-btn {
  width: 32px;
  height: 32px;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
}
</style>
