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
  </div>
</template>

<style scoped>
.message-item {
  display: flex;
  align-items: flex-start;
  margin-bottom: 24px;
}

.message-item.user {
  justify-content: flex-end;
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
  background: linear-gradient(135deg, #6b7280, #4b5563);
  color: #fff;
  border-radius: 14px 14px 4px 14px;
  box-shadow: 0 2px 8px rgba(107, 114, 128, 0.2);
}

/* AI bubble */
.bubble-ai {
  background: #f8fafc;
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 14px 14px 14px 4px;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.04);
}

.dark .bubble-ai {
  background: #1a1a20;
  border-color: rgba(255, 255, 255, 0.08);
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
  background: #6b7280;
  animation: blink 1.4s infinite both;
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
