<script setup lang="ts">
import { onMounted } from "vue";
import { NConfigProvider, NMessageProvider, zhCN, dateZhCN } from "naive-ui";
import { useThemeStore } from "./stores/theme";
import { useConfigStore } from "./stores/config";

const themeStore = useThemeStore();
const configStore = useConfigStore();

onMounted(async () => {
  // Config is auto-loaded from localStorage in the store definition.
  // Try env vars as fallback for API keys.
  await configStore.initFromEnv();
});
</script>

<template>
  <NConfigProvider
    :theme="themeStore.theme"
    :theme-overrides="themeStore.themeOverrides"
    :locale="zhCN"
    :date-locale="dateZhCN"
  >
    <NMessageProvider>
      <RouterView />
    </NMessageProvider>
  </NConfigProvider>
</template>

<style>
html,
body,
#app {
  margin: 0;
  padding: 0;
  height: 100%;
  width: 100%;
  overflow: hidden;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
    "Helvetica Neue", sans-serif;
  transition: background-color 0.3s ease, color 0.3s ease;
}

/* Contrast is applied dynamically via JS only when modified (≠ 100),
   to avoid a permanent GPU compositing layer that blurs all text.
   See theme.ts applyContrast() — toggles filter on #app on demand. */

* {
  box-sizing: border-box;
}

::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.15);
  border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
  background: rgba(0, 0, 0, 0.25);
}

/* Dark mode scrollbar */
html.dark ::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.12);
}

html.dark ::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.2);
}

/* ── Naive UI 下拉菜单全局统一样式 ── */
/* NSelect / NDropdown 内部通过 Teleport 将菜单渲染到 body，
   组件级 scoped 样式无法穿透，因此使用全局非 scoped 样式控制。
   视觉规范与左侧侧边栏右键菜单（Sidebar.vue .context-menu）保持一致。 */

/* ──── NSelect 选择器下拉（模型选择等） ──── */

/* 菜单面板：仅控制外观，不加 padding（会干扰虚拟列表滚动计算） */
.n-base-select-menu {
  background: #ffffff;
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.1), 0 1px 4px rgba(0, 0, 0, 0.06);
  overflow: hidden;
}

/* 选项项：通过 margin 做出 4px 呼吸间距，不依赖父级 padding */
.n-base-select-option {
  margin: 2px 4px;
  padding: 6px 8px;
  font-size: 13px;
  color: #374151;
  border-radius: 5px;
  transition: background-color 0.12s ease;
}

.n-base-select-option:first-child {
  margin-top: 4px;
}

.n-base-select-option:last-child {
  margin-bottom: 4px;
}

.n-base-select-option:hover,
.n-base-select-option.n-base-select-option--pending {
  background: rgba(0, 0, 0, 0.04);
}

.n-base-select-option.n-base-select-option--selected {
  background: rgba(0, 0, 0, 0.04);
  font-weight: 500;
}

/* 平滑滚动 + 细滚动条 */
.n-base-select-menu :where(:not(.n-base-select-option)) {
  scroll-behavior: smooth;
}

.n-base-select-menu ::-webkit-scrollbar {
  width: 5px;
}

.n-base-select-menu ::-webkit-scrollbar-track {
  background: transparent;
}

.n-base-select-menu ::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.12);
  border-radius: 3px;
}

html.dark .n-base-select-menu ::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.1);
}

/* 暗色模式 — 选项 */
html.dark .n-base-select-option {
  color: #e4e4e7;
}

html.dark .n-base-select-option:hover,
html.dark .n-base-select-option.n-base-select-option--pending {
  background: rgba(255, 255, 255, 0.06);
}

html.dark .n-base-select-option.n-base-select-option--selected {
  background: rgba(255, 255, 255, 0.06);
  font-weight: 500;
}

/* ──── NDropdown 通用下拉（项目头 + 号等） ──── */

.n-dropdown-menu {
  background: #ffffff;
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.1), 0 1px 4px rgba(0, 0, 0, 0.06);
  padding: 4px;
  overflow: hidden;
}

.n-dropdown-option {
  padding: 7px 10px;
  font-size: 13px;
  color: #374151;
  border-radius: 5px;
  transition: background-color 0.12s ease;
}

.n-dropdown-option:hover,
.n-dropdown-option.n-dropdown-option--pending {
  background: rgba(0, 0, 0, 0.04);
}

/* ──── 暗色模式 ──── */

html.dark .n-base-select-menu,
html.dark .n-dropdown-menu {
  background: #25252b;
  border-color: rgba(255, 255, 255, 0.08);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4), 0 1px 4px rgba(0, 0, 0, 0.2);
}

html.dark .n-base-select-option,
html.dark .n-dropdown-option {
  color: #e4e4e7;
}

html.dark .n-base-select-option:hover,
html.dark .n-base-select-option.n-base-select-option--pending,
html.dark .n-dropdown-option:hover,
html.dark .n-dropdown-option.n-dropdown-option--pending {
  background: rgba(255, 255, 255, 0.06);
}

html.dark .n-base-select-option.n-base-select-option--selected {
  background: rgba(255, 255, 255, 0.06);
  font-weight: 500;
}
</style>
