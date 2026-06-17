<script setup lang="ts">
import { computed } from "vue";
import { useAgentStore } from "../../stores/agent";
import { useThemeStore } from "../../stores/theme";

const agentStore = useAgentStore();
const themeStore = useThemeStore();

const statusText = computed(() => {
  return agentStore.isRunning ? "\u{1F504} 生成中..." : "⚡ 就绪";
});

const appVersion = "v1.0.2";
</script>

<template>
  <div class="status-bar" :class="{ dark: themeStore.isDark }">
    <div class="status-left">
      <span class="project-label">
        <svg width="14" height="14" viewBox="0 0 16 16" fill="none" class="project-icon-svg">
          <path d="M2 4.5A1.5 1.5 0 013.5 3h2.63a1.5 1.5 0 011.06.44l.77.77a1.5 1.5 0 001.06.44H12.5A1.5 1.5 0 0114 6.15V12.5a1.5 1.5 0 01-1.5 1.5h-9A1.5 1.5 0 012 12.5V4.5z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
        </svg>
        <span>Xuflow 本地</span>
      </span>
    </div>
    <div class="status-right">
      <span class="status-label">{{ statusText }}</span>
      <span class="version-label">{{ appVersion }}</span>
    </div>
  </div>
</template>

<style scoped>
.status-bar {
  height: 30px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  flex-shrink: 0;
  background: #f3f4f6;
  transition: background-color 0.3s ease;
  user-select: none;
}

.status-bar.dark {
  background: #26272B;
}

/* ── Left: status ────────────────────────── */

.status-left {
  display: flex;
  align-items: center;
  flex-shrink: 0;
}

.project-label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  font-weight: 450;
  color: #9ca3af;
  white-space: nowrap;
  letter-spacing: 0.01em;
}

.project-icon-svg {
  color: #9ca3af;
  flex-shrink: 0;
}

.dark .project-label,
.dark .project-icon-svg {
  color: #9ca3af;
}

.status-label {
  font-size: 11px;
  font-weight: 450;
  color: #6b7280;
  white-space: nowrap;
  letter-spacing: 0.01em;
}

.dark .status-label {
  color: #9ca3af;
}

/* ── Right: version ──────────────────────── */

.status-right {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}

.version-label {
  font-size: 11px;
  font-weight: 450;
  color: #9ca3af;
  white-space: nowrap;
  letter-spacing: 0.02em;
  font-family: "SF Mono", "Cascadia Code", "Fira Code", "JetBrains Mono", monospace;
}

.dark .version-label {
  color: #6b7280;
}
</style>
