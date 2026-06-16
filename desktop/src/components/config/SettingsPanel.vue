<script setup lang="ts">
import { NCard, NForm, NFormItem, NInput, NButton, NText } from "naive-ui";
import { ref, watch, onBeforeUnmount } from "vue";
import { useConfigStore, ALL_MODELS } from "../../stores/config";
import { useAgentStore } from "../../stores/agent";
import { useThemeStore } from "../../stores/theme";

const props = defineProps<{ activeSection: string }>();

const store = useConfigStore();
const agentStore = useAgentStore();
const themeStore = useThemeStore();

const applied = ref(false);

// 每个火山引擎模型的 endpoint 输入（本地副本，实时同步回 store）
const endpointInputs = ref<Record<string, string>>({});
for (const m of ALL_MODELS.filter((m) => m.provider === "volcengine")) {
  endpointInputs.value[m.value] = store.getModelEndpoint(m.value) || "";
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
    <!-- API 密钥 -->
    <div v-if="props.activeSection === 'api-keys'" class="section">
      <h2 class="section-title">API 密钥</h2>
      <p class="section-desc">管理你的 DeepSeek 与火山引擎 API 密钥，以及当前使用的模型。</p>
      <NCard class="settings-card">
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
              <span class="feedback-text">API 发送: {{ store.activeApiModelId }}</span>
            </template>
          </NFormItem>
          <NFormItem>
            <NButton :type="applied ? 'success' : 'primary'" @click="applyToBackend" class="apply-btn">
              {{ applied ? '已应用' : '应用配置' }}
            </NButton>
          </NFormItem>
        </NForm>
      </NCard>
    </div>

    <!-- 接入点管理 -->
    <div v-if="props.activeSection === 'endpoints'" class="section">
      <h2 class="section-title">接入点管理</h2>
      <p class="section-desc">
        从 <a href="https://console.volcengine.com/ark/region:ark+cn-beijing/inference" target="_blank">Ark 控制台</a> 复制每个模型的接入点 ID 填入下方。选择模型时将自动使用对应的接入点。输入即时保存，无需手动确认。
      </p>
      <NCard class="settings-card">
        <template #header>
          <div class="card-header">
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
              <rect x="2" y="2" width="7" height="7" rx="1.5" stroke="currentColor" stroke-width="1.6" />
              <rect x="11" y="2" width="7" height="7" rx="1.5" stroke="currentColor" stroke-width="1.6" />
              <rect x="2" y="11" width="7" height="7" rx="1.5" stroke="currentColor" stroke-width="1.6" />
              <rect x="11" y="11" width="7" height="7" rx="1.5" stroke="currentColor" stroke-width="1.6" />
            </svg>
            <span>火山引擎接入点</span>
          </div>
        </template>
        <NForm label-placement="left" label-width="160">
          <NFormItem v-for="m in store.volcModels" :key="m.value" :label="m.label">
            <NInput
              v-model:value="endpointInputs[m.value]"
              :placeholder="`ep-xxxxxxxxxxxx`"
              size="small"
            />
          </NFormItem>
        </NForm>
      </NCard>
    </div>

    <!-- 关于 -->
    <div v-if="props.activeSection === 'about'" class="section">
      <h2 class="section-title">关于</h2>
      <p class="section-desc">Xuflow 的版本与项目信息。</p>
      <NCard class="settings-card">
        <template #header>
          <div class="card-header">
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
              <path d="M10 18V6M6 10l4-4 4 4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
              <path d="M3 3h14" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
            </svg>
            <span>版本信息</span>
          </div>
        </template>
        <div class="about-content">
          <p>Xuflow Desktop v0.1.0</p>
          <p class="about-desc">一个基于 AI 的编程助手工具，支持 CLI 和桌面双端使用。</p>
        </div>
      </NCard>
    </div>
  </div>
</template>

<style scoped>
.settings-panel {
  padding: 32px;
}

.section {
  max-width: 680px;
}

.section-title {
  font-size: 22px;
  font-weight: 700;
  margin: 0 0 6px;
  color: #1e293b;
}

.dark .section-title {
  color: #e2e8f0;
}

.section-desc {
  font-size: 14px;
  color: #64748b;
  margin: 0 0 20px;
  line-height: 1.6;
}

.section-desc a {
  color: #6b7280;
}

.dark .section-desc {
  color: #94a3b8;
}

.settings-card {
  border-radius: 12px;
}

.card-header {
  display: flex;
  align-items: center;
  gap: 10px;
  font-weight: 600;
  flex-wrap: wrap;
}

.card-header svg {
  color: #6b7280;
}

.apply-btn {
  min-width: 100px;
}

.feedback-text {
  font-size: 12px;
  color: #6b7280;
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
