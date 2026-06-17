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
    <!-- ── User message: minimal right-aligned pill ── -->
    <template v-if="isUser">
      <div class="user-pill">
        <span class="user-pill-text">{{ message.content }}</span>
      </div>
    </template>

    <!-- ── AI message: left-aligned, no bubble, pure structured text ── -->
    <template v-else-if="isAssistant">
      <div class="agent-block">
        <StreamText
          :text="message.content"
          :done="message.done"
        />
        <!-- Typing indicator when streaming empty -->
        <div
          v-if="!message.content && !message.done"
          class="typing-indicator"
        >
          <span class="dot"></span>
          <span class="dot"></span>
          <span class="dot"></span>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
/* ── Message container ── */
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

/* ── User pill — minimal tag, no bubble ── */
.user-pill {
  max-width: 75%;
  background: #2E3036;
  padding: 10px 18px;
  border-radius: 10px;
  transition: background 0.2s ease;
}

.dark .user-pill {
  background: #2E3036;
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

/* ── Agent block — full-width structured text ── */
.agent-block {
  width: 100%;
  padding: 0;
  /* no bubble, no border, no background */
}

/* ── Typing indicator ── */
.typing-indicator {
  display: flex;
  gap: 6px;
  padding: 8px 0;
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
