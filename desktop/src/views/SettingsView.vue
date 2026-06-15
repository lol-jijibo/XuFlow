<script setup lang="ts">
import { NLayout, NLayoutContent } from "naive-ui";
import Sidebar from "../components/layout/Sidebar.vue";
import SettingsPanel from "../components/config/SettingsPanel.vue";
import TitleBar from "../components/layout/TitleBar.vue";
import StatusBar from "../components/layout/StatusBar.vue";
import { useThemeStore } from "../stores/theme";

const themeStore = useThemeStore();
</script>

<template>
  <div class="settings-root" :class="{ dark: themeStore.isDark }">
    <Sidebar />
    <div class="settings-main">
      <NLayout
        class="settings-layout"
        content-style="display:flex;flex-direction:column;"
      >
        <TitleBar />
        <NLayoutContent class="settings-content">
          <SettingsPanel />
        </NLayoutContent>
        <StatusBar />
      </NLayout>
    </div>
  </div>
</template>

<style scoped>
.settings-root {
  display: flex;
  height: 100vh;
  overflow: hidden;
  background: #ffffff;
  transition: background-color 0.3s ease;
}

.settings-root.dark {
  background: #101014;
}

.settings-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.settings-layout {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.settings-content {
  flex: 1;
  padding: 24px;
  overflow-y: auto;
}
</style>
