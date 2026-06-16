<script setup lang="ts">
import { ref, h } from "vue";
import { useRouter } from "vue-router";
import { NLayout, NLayoutSider, NLayoutContent, NMenu, NButton } from "naive-ui";
import type { MenuOption } from "naive-ui";
import SettingsPanel from "../components/config/SettingsPanel.vue";
import { useThemeStore } from "../stores/theme";

const router = useRouter();
const themeStore = useThemeStore();

const activeSection = ref("api-keys");

function renderIcon(icon: string) {
  return () => h("span", { class: "menu-icon", innerHTML: icon });
}

const keyIcon = `<svg width="20" height="20" viewBox="0 0 20 20" fill="none">
  <path d="M12.5 7.5a2.5 2.5 0 10-5 0 2.5 2.5 0 005 0z" stroke="currentColor" stroke-width="1.6"/>
  <path d="M12.5 9.5v3a1 1 0 01-1 1h-1l-1 2H8M4 10V8a6 6 0 1112 0v2" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/>
</svg>`;

const gridIcon = `<svg width="20" height="20" viewBox="0 0 20 20" fill="none">
  <rect x="2" y="2" width="7" height="7" rx="1.5" stroke="currentColor" stroke-width="1.6"/>
  <rect x="11" y="2" width="7" height="7" rx="1.5" stroke="currentColor" stroke-width="1.6"/>
  <rect x="2" y="11" width="7" height="7" rx="1.5" stroke="currentColor" stroke-width="1.6"/>
  <rect x="11" y="11" width="7" height="7" rx="1.5" stroke="currentColor" stroke-width="1.6"/>
</svg>`;

const infoIcon = `<svg width="20" height="20" viewBox="0 0 20 20" fill="none">
  <circle cx="10" cy="10" r="8" stroke="currentColor" stroke-width="1.6"/>
  <path d="M10 9v5M10 6v1" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/>
</svg>`;

const menuOptions: MenuOption[] = [
  { label: "API 密钥", key: "api-keys", icon: renderIcon(keyIcon) },
  { label: "接入点管理", key: "endpoints", icon: renderIcon(gridIcon) },
  { label: "关于", key: "about", icon: renderIcon(infoIcon) },
];
</script>

<template>
  <div class="settings-root" :class="{ dark: themeStore.isDark }">
    <!-- 顶栏：返回 + 标题 -->
    <div class="settings-topbar">
      <NButton quaternary circle size="small" @click="router.push('/')" title="返回聊天">
        <template #icon>
          <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
            <path d="M11 4l-5 5 5 5" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </template>
      </NButton>
      <span class="topbar-title">设置</span>
    </div>

    <!-- 双栏布局 -->
    <NLayout class="settings-layout" has-sider>
      <NLayoutSider width="240" class="settings-sider">
        <NMenu
          :value="activeSection"
          :options="menuOptions"
          @update:value="(val) => activeSection = val"
        />
      </NLayoutSider>
      <NLayoutContent class="settings-content">
        <SettingsPanel :active-section="activeSection" />
      </NLayoutContent>
    </NLayout>
  </div>
</template>

<style scoped>
.settings-root {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100vw;
  overflow: hidden;
  background: #ffffff;
  transition: background-color 0.3s ease;
}

.settings-root.dark {
  background: #101014;
}

/* 顶栏 */
.settings-topbar {
  display: flex;
  align-items: center;
  gap: 12px;
  height: 48px;
  padding: 0 16px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
  flex-shrink: 0;
  -webkit-app-region: drag;
}

.settings-topbar button {
  -webkit-app-region: no-drag;
}

.dark .settings-topbar {
  border-bottom-color: rgba(255, 255, 255, 0.06);
}

.topbar-title {
  font-size: 16px;
  font-weight: 600;
  color: #1e293b;
}

.dark .topbar-title {
  color: #e2e8f0;
}

/* 双栏 */
.settings-layout {
  flex: 1;
  min-height: 0;
}

.settings-sider {
  border-right: 1px solid rgba(0, 0, 0, 0.06);
  background: #fafafa;
  transition: background-color 0.3s ease, border-color 0.3s ease;
}

.dark .settings-sider {
  background: #1c1c22;
  border-right-color: rgba(255, 255, 255, 0.06);
}

.settings-content {
  flex: 1;
  overflow-y: auto;
  background: #ffffff;
  transition: background-color 0.3s ease;
}

.dark .settings-content {
  background: #101014;
}

/* NMenu 微调 */
.settings-sider :deep(.n-menu) {
  padding-top: 8px;
}

.settings-sider :deep(.n-menu-item-content) {
  margin: 2px 8px;
  border-radius: 8px;
}

/* 菜单图标 */
:deep(.menu-icon) {
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>
