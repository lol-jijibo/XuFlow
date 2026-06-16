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
</style>
