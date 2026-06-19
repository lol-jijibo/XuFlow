<script setup lang="ts">
import { computed } from "vue";
import { useAgentStore } from "../../stores/agent";
import { useThemeStore } from "../../stores/theme";
import { NScrollbar } from "naive-ui";

const store = useAgentStore();
const themeStore = useThemeStore();

const visible = computed(() => store.todos.length > 0);

const completedCount = computed(() => store.todos.filter(t => t.status === "completed").length);
const totalCount = computed(() => store.todos.length);

function statusIcon(status: string): string {
  switch (status) {
    case "completed": return "✓";
    case "in_progress": return "●";
    default: return "○";
  }
}
</script>

<template>
  <div v-if="visible" class="todo-panel" :class="{ dark: themeStore.isDark }">
    <div class="todo-header">
      <span class="todo-title">任务进度</span>
      <span class="todo-count">{{ completedCount }}/{{ totalCount }}</span>
    </div>
    <NScrollbar class="todo-scroll" style="max-height: 200px">
      <div class="todo-list">
        <div
          v-for="(item, idx) in store.todos"
          :key="idx"
          class="todo-item"
          :class="{ completed: item.status === 'completed', active: item.status === 'in_progress' }"
        >
          <span class="todo-status">{{ statusIcon(item.status) }}</span>
          <span class="todo-content">{{ item.content }}</span>
        </div>
      </div>
    </NScrollbar>
  </div>
</template>

<style scoped>
.todo-panel {
  margin: 0 44px 16px;
  background: rgba(255, 255, 255, 0.64);
  backdrop-filter: blur(12px);
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 14px;
  overflow: hidden;
  transition: background 0.3s ease, border-color 0.3s ease;
}

.todo-panel.dark {
  background: rgba(32, 32, 38, 0.8);
  border-color: rgba(255, 255, 255, 0.06);
}

.todo-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px 8px;
}

.todo-title {
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: #6b7280;
}

.dark .todo-title {
  color: #9ca3af;
}

.todo-count {
  font-size: 11px;
  font-weight: 600;
  color: #9ca3af;
  font-variant-numeric: tabular-nums;
}

.dark .todo-count {
  color: #6b7280;
}

.todo-scroll {
  padding: 0 8px 10px;
}

.todo-item {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 6px 8px;
  border-radius: 8px;
  font-size: 13px;
  transition: background 0.12s ease;
}

.todo-item:hover {
  background: rgba(0, 0, 0, 0.03);
}

.dark .todo-item:hover {
  background: rgba(255, 255, 255, 0.03);
}

.todo-item.completed .todo-content {
  text-decoration: line-through;
  color: #9ca3af;
}

.dark .todo-item.completed .todo-content {
  color: #6b7280;
}

.todo-item.active {
  background: rgba(99, 102, 241, 0.06);
}

.dark .todo-item.active {
  background: rgba(129, 140, 248, 0.08);
}

.todo-item.active .todo-content {
  color: #6366f1;
  font-weight: 500;
}

.dark .todo-item.active .todo-content {
  color: #a5b4fc;
}

.todo-status {
  flex-shrink: 0;
  width: 16px;
  text-align: center;
  font-size: 11px;
  color: #9ca3af;
}

.todo-item.active .todo-status {
  color: #6366f1;
}

.dark .todo-item.active .todo-status {
  color: #a5b4fc;
}

.todo-item.completed .todo-status {
  color: #22c55e;
}

.todo-content {
  flex: 1;
  line-height: 1.45;
  color: #374151;
}

.dark .todo-content {
  color: #d1d5db;
}
</style>
