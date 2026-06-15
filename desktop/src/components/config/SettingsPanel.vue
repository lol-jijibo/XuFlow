<script setup lang="ts">
import { NCard, NForm, NFormItem, NInput, NButton, NText, NTag, NCollapse, NCollapseItem } from "naive-ui";
import { ref, watch, onBeforeUnmount } from "vue";
import { useRouter } from "vue-router";
import { useConfigStore, ALL_MODELS } from "../../stores/config";
import { useAgentStore } from "../../stores/agent";
import { useThemeStore } from "../../stores/theme";

const router = useRouter();
const store = useConfigStore();
const agentStore = useAgentStore();
const themeStore = useThemeStore();

const applied = ref(false);

// 每个火山引擎模型的 endpoint 输入（本地副本，实时同步回 store）
const endpointInputs = ref<Record<string, string>>({});
for (const m of ALL_MODELS.filter((m) => m.provider === "volcengine")) {
  endpointInputs.value[m.value] = store.getModelEndpoint(m.value) || m.apiModelId;
}

// 实时将 endpoint 变更写回 store（每输入即自动保存）
watch(
  endpointInputs,
  (newVal) => {
    for (const [modelValue, ep] of Object.entries(newVal)) {
      store.setModelEndpoint(modelValue, ep.trim());
    }
  },
  { deep: true }
);

// 组件卸载时确保最新值已写入 store
onBeforeUnmount(() => {
  for (const [modelValue, ep] of Object.entries(endpointInputs.value)) {
    store.setModelEndpoint(modelValue, ep.trim());
  }
});

// 将配置推送到 Rust 后端
async function applyToBackend() {
  await agentStore.configureAgent();
  applied.value = true;
  setTimeout(() => { applied.value = false; }, 2000);
}
</script>

<template>
  <div class="settings-panel" :class="{ dark: themeStore.isDark }">
    <div class="settings-header">
      <div class="settings-header-row">
        <div>
          <h1 class="settings-title">设置</h1>
          <p class="settings-subtitle">管理你的 API 密钥和偏好设置——所有改动自动保存</p>
        </div>
        <NButton quaternary circle size="small" class="close-btn" @click="router.push('/')" title="返回聊天">
          <template #icon>
            <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
              <path d="M5 5l8 8M13 5l-8 8" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
            </svg>
          </template>
        </NButton>
      </div>
    </div>

    <NCard class="settings-card">
      <template #header>
        <div class="card-header">
          <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
            <path d="M15 7a3 3 0 100-6 3 3 0 000 6zM2.5 7h6M2.5 13h15M2.5 16h10" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
            <path d="M15 7v10M12 12h6" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
          </svg>
          <span>API 配置</span>
        </div>
      </template>
      <NForm label-placement="left" label-width="100">
        <NFormItem label="DeepSeek Key">
          <NInput v-model:value="store.deepseekApiKey" type="password" placeholder="输入 DeepSeek API Key" show-password-on="click" />
        </NFormItem>
        <NFormItem label="火山引擎 Key">
          <NInput v-model:value="store.volcengineApiKey" type="password" placeholder="输入火山引擎 API Key" show-password-on="click" />
        </NFormItem>
        <NFormItem label="当前模型">
          <NText>{{ store.activeModelName }} ({{ store.activeProvider }})</NText>
          <template #feedback>
            <span style="font-size: 12px; color: #6366f1;">API 发送: {{ store.activeApiModelId }}</span>
          </template>
        </NFormItem>
        <NFormItem>
          <NButton :type="applied ? 'success' : 'primary'" @click="applyToBackend" class="apply-btn">
            {{ applied ? '已应用' : '应用配置' }}
          </NButton>
        </NFormItem>
      </NForm>
    </NCard>

    <!-- 火山引擎接入点 · 可折叠 -->
    <NCard class="settings-card">
      <NCollapse :default-expanded-names="[]" accordion>
        <NCollapseItem name="endpoints">
          <template #header>
            <div class="card-header collapse-header">
              <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
                <rect x="2" y="2" width="7" height="7" rx="1.5" stroke="currentColor" stroke-width="1.6" />
                <rect x="11" y="2" width="7" height="7" rx="1.5" stroke="currentColor" stroke-width="1.6" />
                <rect x="2" y="11" width="7" height="7" rx="1.5" stroke="currentColor" stroke-width="1.6" />
                <rect x="11" y="11" width="7" height="7" rx="1.5" stroke="currentColor" stroke-width="1.6" />
              </svg>
              <span>火山引擎接入点</span>
              <NTag size="small" type="info" :bordered="false">展开配置</NTag>
            </div>
          </template>
          <div class="endpoint-hint">
            从 <a href="https://console.volcengine.com/ark/region:ark+cn-beijing/inference" target="_blank">Ark 控制台</a> 复制每个模型的接入点 ID 填入下方。选择模型时将自动使用对应的接入点。输入即时保存，无需手动确认。
          </div>
          <NForm label-placement="left" label-width="160">
            <NFormItem v-for="m in store.volcModels" :key="m.value" :label="m.label">
              <NInput
                v-model:value="endpointInputs[m.value]"
                :placeholder="`ep-xxxxxxxxxxxx`"
                size="small"
              />
            </NFormItem>
          </NForm>
        </NCollapseItem>
      </NCollapse>
    </NCard>

    <NCard class="settings-card">
      <template #header>
        <div class="card-header">
          <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
            <path d="M10 18V6M6 10l4-4 4 4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
            <path d="M3 3h14" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
          </svg>
          <span>关于</span>
        </div>
      </template>
      <div class="about-content">
        <p>Xuflow Desktop v0.1.0</p>
        <p class="about-desc">一个基于 AI 的编程助手工具，支持 CLI 和桌面双端使用。</p>
      </div>
    </NCard>
  </div>
</template>

<style scoped>
.settings-panel {
  max-width: 700px;
  margin: 0 auto;
}
.settings-header { margin-bottom: 24px; }
.settings-header-row { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; }
.close-btn { flex-shrink: 0; color: #94a3b8; transition: color 0.15s ease; }
.close-btn:hover { color: #ef4444; }
.dark .close-btn { color: #64748b; }
.dark .close-btn:hover { color: #f87171; }
.settings-title { font-size: 24px; font-weight: 700; margin: 0 0 4px; color: #1e293b; }
.dark .settings-title { color: #e2e8f0; }
.settings-subtitle { font-size: 14px; color: #64748b; margin: 0; }
.settings-card { margin-bottom: 16px; border-radius: 12px; }
.card-header { display: flex; align-items: center; gap: 10px; font-weight: 600; flex-wrap: wrap; }
.card-header svg { color: #6366f1; }
.apply-btn { min-width: 100px; }
.about-content p { margin: 0 0 8px; font-size: 14px; }
.about-desc { color: #64748b; }
.dark .about-desc { color: #94a3b8; }
.endpoint-hint { font-size: 13px; color: #64748b; margin-bottom: 16px; line-height: 1.5; }
.endpoint-hint a { color: #6366f1; }

.collapse-header { margin: 0; }

/* 平滑折叠动画 */
.settings-card :deep(.n-collapse-item__content-wrapper) {
  transition: height 0.3s cubic-bezier(0.4, 0, 0.2, 1) !important;
}
.settings-card :deep(.n-collapse-item-arrow) {
  transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1) !important;
}
</style>
