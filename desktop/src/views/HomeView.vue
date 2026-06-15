<script setup lang="ts">
import { NLayout, NLayoutContent } from "naive-ui";
import Sidebar from "../components/layout/Sidebar.vue";
import ChatPanel from "../components/chat/ChatPanel.vue";
import TitleBar from "../components/layout/TitleBar.vue";
import StatusBar from "../components/layout/StatusBar.vue";
import ApprovalModal from "../components/approval/ApprovalModal.vue";
import { useThemeStore } from "../stores/theme";

const themeStore = useThemeStore();
</script>

<template>
  <div class="home-root" :class="{ dark: themeStore.isDark }">
    <Sidebar />
    <div class="home-main">
      <NLayout class="home-layout" content-style="display:flex;flex-direction:column;">
        <TitleBar />
        <NLayoutContent class="home-content">
          <ChatPanel />
        </NLayoutContent>
        <StatusBar />
      </NLayout>
    </div>
    <ApprovalModal />
  </div>
</template>

<style scoped>
.home-root {
  display: flex;
  height: 100vh;
  overflow: hidden;
  background: #ffffff;
  transition: background-color 0.3s ease;
}

.home-root.dark {
  background: #101014;
}

.home-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.home-layout {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.home-content {
  flex: 1;
  overflow: hidden;
}
</style>
