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

const isEmpty = computed(() => store.messages.length === 0);

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
    console.error("[chat] No active conversation — cannot send message");
    return;
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
      <div class="input-container">
        <div class="input-wrapper">
          <NInput
            v-model:value="inputText"
            type="textarea"
            placeholder="输入消息... (Enter 发送, Shift+Enter 换行)"
            :autosize="{ minRows: 1, maxRows: 6 }"
            :disabled="store.isRunning"
            @keydown.enter.exact.prevent="sendMessage"
            class="chat-input"
          />
          <div class="input-actions">
            <NButton
              v-if="store.isRunning"
              type="warning"
              size="small"
              quaternary
              @click="handleStop"
            >
              <template #icon>
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                  <rect
                    x="3"
                    y="3"
                    width="10"
                    height="10"
                    rx="2"
                    fill="currentColor"
                  />
                </svg>
              </template>
              停止
            </NButton>
            <NButton
              v-else
              type="primary"
              size="small"
              :loading="sending"
              :disabled="!inputText.trim()"
              @click="sendMessage"
              class="send-btn"
            >
              <template #icon>
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                  <path
                    d="M14 2L7 9M14 2l-4 12-3-5-5-3 12-4z"
                    stroke="currentColor"
                    stroke-width="1.4"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  />
                </svg>
              </template>
            </NButton>
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
  background: #ffffff;
  transition: background-color 0.3s ease;
}

.chat-panel.dark {
  background: #1c1c22;
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
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.2s ease;
  text-align: left;
  font-size: 14px;
  color: #475569;
}

.prompt-card:hover {
  background: #f1f5f9;
  border-color: #6366f1;
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(99, 102, 241, 0.15);
}

.chat-panel.dark .prompt-card {
  background: #1c1c22;
  border-color: rgba(255, 255, 255, 0.1);
  color: #94a3b8;
}

.chat-panel.dark .prompt-card:hover {
  background: #28282f;
  border-color: #6366f1;
  box-shadow: 0 4px 12px rgba(99, 102, 241, 0.2);
}

.prompt-card svg {
  flex-shrink: 0;
  color: #6366f1;
}

/* Messages */
.chat-scroll {
  flex: 1;
}

.message-list {
  padding: 24px;
  max-width: 860px;
  margin: 0 auto;
}

/* Input area */
.chat-footer {
  padding: 16px 24px 20px;
  border-top: 1px solid rgba(0, 0, 0, 0.06);
  background: #fafafa;
  transition: background-color 0.3s ease, border-color 0.3s ease;
}

.chat-panel.dark .chat-footer {
  background: #1c1c22;
  border-top-color: rgba(255, 255, 255, 0.06);
}

.input-container {
  max-width: 860px;
  margin: 0 auto;
}

.input-wrapper {
  display: flex;
  align-items: flex-end;
  gap: 8px;
  background: #ffffff;
  border: 1px solid rgba(0, 0, 0, 0.1);
  border-radius: 12px;
  padding: 8px 12px;
  transition: all 0.2s ease;
}

.input-wrapper:focus-within {
  border-color: #6366f1;
  box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.1);
}

.chat-panel.dark .input-wrapper {
  background: #1c1c22;
  border-color: rgba(255, 255, 255, 0.1);
}

.chat-panel.dark .input-wrapper:focus-within {
  border-color: #6366f1;
  box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.2);
}

.chat-input {
  flex: 1;
}

.chat-input :deep(.n-input__border),
.chat-input :deep(.n-input__state-border) {
  display: none;
}

.chat-input :deep(.n-input-wrapper) {
  padding: 0;
  background: transparent;
}

.chat-input :deep(.n-input__textarea-el) {
  resize: none;
  font-size: 14px;
  line-height: 1.5;
}

.input-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
  padding-bottom: 2px;
}

.send-btn {
  width: 32px;
  height: 32px;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>
