<script setup lang="ts">
import { NCard, NForm, NFormItem, NInput, NButton, NText } from "naive-ui";
import { ref } from "vue";
import { useConfigStore } from "../../stores/config";
import { useThemeStore } from "../../stores/theme";

const store = useConfigStore();
const themeStore = useThemeStore();
const apiKey = ref("");

function save() {
  if (apiKey.value.trim()) {
    store.setApiKey(apiKey.value.trim());
  }
}
</script>

<template>
  <div class="settings-panel" :class="{ dark: themeStore.isDark }">
    <div class="settings-header">
      <h1 class="settings-title">设置</h1>
      <p class="settings-subtitle">管理你的 API 密钥和偏好设置</p>
    </div>

    <NCard class="settings-card">
      <template #header>
        <div class="card-header">
          <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
            <path
              d="M15 7a3 3 0 100-6 3 3 0 000 6zM2.5 7h6M2.5 13h15M2.5 16h10"
              stroke="currentColor"
              stroke-width="1.6"
              stroke-linecap="round"
            />
            <path
              d="M15 7v10M12 12h6"
              stroke="currentColor"
              stroke-width="1.6"
              stroke-linecap="round"
            />
          </svg>
          <span>API 配置</span>
        </div>
      </template>
      <NForm label-placement="left" label-width="80">
        <NFormItem label="API Key">
          <NInput
            v-model:value="apiKey"
            type="password"
            placeholder="输入你的 API Key"
            show-password-on="click"
          />
        </NFormItem>
        <NFormItem label="当前模型">
          <NText>{{ store.activeModelId }}</NText>
        </NFormItem>
        <NFormItem>
          <NButton type="primary" @click="save" class="save-btn">
            保存设置
          </NButton>
        </NFormItem>
      </NForm>
    </NCard>

    <NCard class="settings-card">
      <template #header>
        <div class="card-header">
          <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
            <path
              d="M10 18V6M6 10l4-4 4 4"
              stroke="currentColor"
              stroke-width="1.6"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
            <path
              d="M3 3h14"
              stroke="currentColor"
              stroke-width="1.6"
              stroke-linecap="round"
            />
          </svg>
          <span>关于</span>
        </div>
      </template>
      <div class="about-content">
        <p>Xuflow Desktop v0.1.0</p>
        <p class="about-desc">
          一个基于 AI 的编程助手工具，支持 CLI 和桌面双端使用。
        </p>
      </div>
    </NCard>
  </div>
</template>

<style scoped>
.settings-panel {
  max-width: 640px;
  margin: 0 auto;
}

.settings-header {
  margin-bottom: 24px;
}

.settings-title {
  font-size: 24px;
  font-weight: 700;
  margin: 0 0 4px;
  color: #1e293b;
}

.dark .settings-title {
  color: #e2e8f0;
}

.settings-subtitle {
  font-size: 14px;
  color: #64748b;
  margin: 0;
}

.settings-card {
  margin-bottom: 16px;
  border-radius: 12px;
}

.card-header {
  display: flex;
  align-items: center;
  gap: 10px;
  font-weight: 600;
}

.card-header svg {
  color: #6366f1;
}

.save-btn {
  min-width: 100px;
}

.about-content p {
  margin: 0 0 8px;
  font-size: 14px;
}

.about-desc {
  color: #64748b;
}

.dark .about-desc {
  color: #94a3b8;
}
</style>
