<script setup lang="ts">
import { NText } from "naive-ui";
import { useAgentStore } from "../../stores/agent";
import { useProjectStore } from "../../stores/project";
import { useThemeStore } from "../../stores/theme";
import { useConfigStore } from "../../stores/config";

const agentStore = useAgentStore();
const projectStore = useProjectStore();
const themeStore = useThemeStore();
const configStore = useConfigStore();
</script>

<template>
  <div class="status-bar" :class="{ dark: themeStore.isDark }">
    <div class="status-left">
      <div class="status-indicator" :class="{ running: agentStore.isRunning }">
        <span class="indicator-dot"></span>
        <NText depth="3" class="status-text">
          {{ agentStore.isRunning ? "运行中" : "就绪" }}
        </NText>
      </div>
    </div>
    <div class="status-right">
      <NText depth="3" class="status-text model-name">
        {{ configStore.activeModelId }}
      </NText>
      <span class="status-divider">|</span>
      <NText depth="3" class="status-text" v-if="projectStore.activeProject">
        {{ projectStore.activeProject.name }}
      </NText>
    </div>
  </div>
</template>

<style scoped>
.status-bar {
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  border-top: 1px solid rgba(0, 0, 0, 0.06);
  background: #fafafa;
  flex-shrink: 0;
  transition: background-color 0.3s ease, border-color 0.3s ease;
}

.status-bar.dark {
  background: #101014;
  border-top-color: rgba(255, 255, 255, 0.06);
}

.status-left,
.status-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-indicator {
  display: flex;
  align-items: center;
  gap: 6px;
}

.indicator-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #94a3b8;
  transition: background-color 0.3s ease;
}

.status-indicator.running .indicator-dot {
  background: #22c55e;
  box-shadow: 0 0 6px rgba(34, 197, 94, 0.5);
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.5;
  }
}

.status-text {
  font-size: 12px;
}

.model-name {
  font-family: monospace;
  font-size: 11px;
}

.status-divider {
  color: #94a3b8;
  font-size: 10px;
}

.dark .status-divider {
  color: #475569;
}
</style>
