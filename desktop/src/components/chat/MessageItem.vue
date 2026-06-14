<script setup lang="ts">
import { computed } from "vue";
import StreamText from "./StreamText.vue";
import { useThemeStore } from "../../stores/theme";
import type { ChatMessage } from "../../stores/project";

const props = defineProps<{
  message: ChatMessage;
}>();

const themeStore = useThemeStore();
const isUser = computed(() => props.message.role === "user");
const isAssistant = computed(() => props.message.role === "assistant");
</script>

<template>
  <div
    class="message-item"
    :class="{ user: isUser, assistant: isAssistant, dark: themeStore.isDark }"
  >
    <!-- AI avatar (left) -->
    <div v-if="isAssistant" class="avatar avatar-ai">
      <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
        <path
          d="M4 5h12M4 10h9M4 15h6"
          stroke="#fff"
          stroke-width="2"
          stroke-linecap="round"
        />
      </svg>
    </div>

    <!-- Bubble -->
    <div
      class="bubble"
      :class="{ 'bubble-user': isUser, 'bubble-ai': isAssistant }"
    >
      <div class="bubble-content" v-if="message.content">
        <StreamText
          v-if="isAssistant"
          :text="message.content"
          :done="message.done"
        />
        <span v-else>{{ message.content }}</span>
      </div>
      <div
        v-if="isAssistant && !message.content && !message.done"
        class="typing-indicator"
      >
        <span class="dot"></span>
        <span class="dot"></span>
        <span class="dot"></span>
      </div>
    </div>

    <!-- User avatar (right) -->
    <div v-if="isUser" class="avatar avatar-user">
      <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
        <path
          d="M15 16.5v-1.5a3 3 0 00-3-3H6a3 3 0 00-3 3v1.5M9 9a3 3 0 100-6 3 3 0 000 6z"
          stroke="currentColor"
          stroke-width="1.6"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </div>
  </div>
</template>

<style scoped>
.message-item {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  margin-bottom: 24px;
}

.message-item.user {
  flex-direction: row-reverse;
}

/* Avatar */
.avatar {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: transform 0.15s ease;
}

.avatar:hover {
  transform: scale(1.05);
}

.avatar-ai {
  background: linear-gradient(135deg, #6366f1, #8b5cf6);
  box-shadow: 0 2px 8px rgba(99, 102, 241, 0.3);
}

.avatar-user {
  background: #f1f5f9;
  color: #475569;
  border: 1px solid rgba(0, 0, 0, 0.06);
}

.dark .avatar-user {
  background: #2d2d5e;
  color: #94a3b8;
  border-color: rgba(255, 255, 255, 0.1);
}

/* Bubble base */
.bubble {
  max-width: 75%;
  padding: 12px 16px;
  line-height: 1.7;
  word-break: break-word;
  transition: all 0.2s ease;
}

.bubble-user {
  max-width: 70%;
}

.bubble-ai {
  max-width: 85%;
}

/* User bubble */
.bubble-user {
  background: linear-gradient(135deg, #6366f1, #7c3aed);
  color: #fff;
  border-radius: 14px 14px 4px 14px;
  box-shadow: 0 2px 8px rgba(99, 102, 241, 0.2);
}

/* AI bubble */
.bubble-ai {
  background: #f8fafc;
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 14px 14px 14px 4px;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.04);
}

.dark .bubble-ai {
  background: #1e1e3f;
  border-color: rgba(255, 255, 255, 0.06);
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.2);
}

.bubble-content {
  font-size: 14px;
}

/* Typing indicator */
.typing-indicator {
  display: flex;
  gap: 6px;
  padding: 6px 0;
}

.dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: #6366f1;
  animation: blink 1.4s infinite both;
}

.dark .dot {
  background: #818cf8;
}

.dot:nth-child(2) {
  animation-delay: 0.2s;
}

.dot:nth-child(3) {
  animation-delay: 0.4s;
}

@keyframes blink {
  0%,
  80%,
  100% {
    opacity: 0.3;
    transform: scale(0.8);
  }
  40% {
    opacity: 1;
    transform: scale(1);
  }
}
</style>
