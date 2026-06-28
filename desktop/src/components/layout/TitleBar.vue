<script setup lang="ts">
import { NButton, NText } from "naive-ui";
import { useProjectStore } from "../../stores/project";
import { useThemeStore } from "../../stores/theme";

const projectStore = useProjectStore();
const themeStore = useThemeStore();
</script>

<template>
  <div class="title-bar" :class="{ dark: themeStore.isDark }">
    <div class="title-left">
      <NText tag="div" class="title-project">
        {{ projectStore.activeProject?.name ?? "Xuflow" }}
      </NText>
    </div>
    <div class="title-right">
      <NButton
        quaternary
        size="small"
        class="theme-toggle"
        @click="themeStore.toggle()"
        :title="themeStore.isDark ? '切换亮色' : '切换暗色'"
      >
        <template #icon>
          <!-- Sun icon for light mode -->
          <svg
            v-if="themeStore.isDark"
            width="18"
            height="18"
            viewBox="0 0 18 18"
            fill="none"
          >
            <circle cx="9" cy="9" r="3.5" stroke="currentColor" stroke-width="1.4" />
            <path d="M9 2v1.5M9 14.5V16M2 9h1.5M14.5 9H16M4.2 4.2l1.06 1.06M12.74 12.74l1.06 1.06M4.2 13.8l1.06-1.06M12.74 5.26l1.06-1.06" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" />
          </svg>
          <!-- Moon icon for dark mode -->
          <svg
            v-else
            width="18"
            height="18"
            viewBox="0 0 18 18"
            fill="none"
          >
            <path d="M15.5 10.5a6.5 6.5 0 01-8-8A6.5 6.5 0 1015.5 10.5z" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round" />
          </svg>
        </template>
      </NButton>
    </div>
  </div>
</template>

<style scoped>
.title-bar {
  height: 44px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
  background: #fafafa;
  -webkit-app-region: drag;
  flex-shrink: 0;
  transition: background-color 0.3s ease, border-color 0.3s ease;
}

.title-bar.dark {
  background: #101014;
  border-bottom-color: rgba(255, 255, 255, 0.06);
}

.title-bar .n-button,
.title-bar .n-select {
  -webkit-app-region: no-drag;
}

.title-left {
  display: flex;
  align-items: baseline;
  gap: 6px;
}

.title-project {
  font-weight: 600;
  font-size: 14px;
}

.title-conv {
  font-size: 13px;
}

.title-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.theme-toggle {
  -webkit-app-region: no-drag;
}
</style>
