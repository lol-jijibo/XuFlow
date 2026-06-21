<script setup lang="ts">
import { NCard, NForm, NFormItem, NInput, NInputNumber, NButton, NText, NSlider } from "naive-ui";
import { ref, watch, onBeforeUnmount, computed } from "vue";
import { useProjectStore } from "../../stores/project";
import { loadDbConfig, saveDbConfig } from "../../stores/config";
import { invoke } from "@tauri-apps/api/core";
import { useConfigStore, ALL_MODELS, type TokenEstimateConfig } from "../../stores/config";
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

// ── Token estimate config helpers ──

const DEFAULT_EST_CONFIG: TokenEstimateConfig = { cjkCoeff: 1.3, nonCjkCoeff: 0.25, structuredCoeff: 0.5 };

function getEstConfig(modelValue: string): TokenEstimateConfig {
  const custom = store.tokenEstimateConfigs[modelValue];
  if (custom) return custom;
  const found = ALL_MODELS.find((m) => m.value === modelValue);
  return found?.tokenEstimateConfig ?? DEFAULT_EST_CONFIG;
}

function updateEstConfig(modelValue: string, patch: Partial<TokenEstimateConfig>) {
  store.setTokenEstimateConfig(modelValue, { ...getEstConfig(modelValue), ...patch });
}

/** Models that support token config (all current models) */
const allModelsForContext = computed(() => ALL_MODELS);

// Template-safe handler wrappers (no TS annotations needed in @update:value)
function handleContextWindowChange(modelValue: string, v: number | null) {
  if (v != null) store.setContextWindow(modelValue, v);
}
function handleCjkCoeffChange(modelValue: string, v: number | null) {
  if (v != null) updateEstConfig(modelValue, { cjkCoeff: v });
}
function handleNonCjkCoeffChange(modelValue: string, v: number | null) {
  if (v != null) updateEstConfig(modelValue, { nonCjkCoeff: v });
}
function handleStructuredCoeffChange(modelValue: string, v: number | null) {
  if (v != null) updateEstConfig(modelValue, { structuredCoeff: v });
}
// Curry-style wrappers to avoid implicit `any` in template arrows
const ctxWinHandler = (m: string) => (v: number | null) => handleContextWindowChange(m, v);
const cjkHandler    = (m: string) => (v: number | null) => handleCjkCoeffChange(m, v);
const nonCjkHandler = (m: string) => (v: number | null) => handleNonCjkCoeffChange(m, v);
const structHandler = (m: string) => (v: number | null) => handleStructuredCoeffChange(m, v);

  // ── 数据库连接配置 ──────────────────────────────────────

  const projectStore = useProjectStore();

  const savedDbConfig = loadDbConfig();
  const dbHost = ref(savedDbConfig?.host ?? "127.0.0.1");
  const dbPort = ref(savedDbConfig?.port ?? 3306);
  const dbUser = ref(savedDbConfig?.user ?? "root");
  const dbPassword = ref(savedDbConfig?.password ?? "");
  const dbName = ref(savedDbConfig?.database ?? "xuflow");

  const dbConnecting = ref(false);
  const dbTestOk = ref(false);
  const dbTestFail = ref(false);
  const dbConnected = ref(false);

  async function checkDbStatus() {
    try {
      const ok = await invoke<boolean>("db_is_connected");
      dbConnected.value = ok;
      if (ok) {
        projectStore.dbConnected = true;
        store.loadFromMySql();
      }
    } catch {
      dbConnected.value = false;
    }
  }

  async function testConnection() {
    dbConnecting.value = true;
    dbTestOk.value = false;
    dbTestFail.value = false;
    try {
      await invoke("db_test_connection", {
        opts: {
          host: dbHost.value,
          port: dbPort.value,
          user: dbUser.value,
          password: dbPassword.value,
          database: dbName.value,
        }
      });
      dbTestOk.value = true;
    } catch (e: any) {
      dbTestFail.value = true;
      console.error("[settings] DB test failed:", e);
    } finally {
      dbConnecting.value = false;
    }
  }

  async function saveAndConnect() {
    dbConnecting.value = true;
    dbTestOk.value = false;
    dbTestFail.value = false;
    saveDbConfig({
      host: dbHost.value,
      port: dbPort.value,
      user: dbUser.value,
      password: dbPassword.value,
      database: dbName.value,
    });
    try {
      await invoke("db_connect", {
        opts: {
          host: dbHost.value,
          port: dbPort.value,
          user: dbUser.value,
          password: dbPassword.value,
          database: dbName.value,
        }
      });
      dbConnected.value = true;
      dbTestOk.value = true;
      projectStore.dbConnected = true;
      await projectStore.tryLoadFromMySql();
      await store.loadFromMySql();
      try {
        const isMigrated = await invoke<boolean>("db_is_migrated");
        if (!isMigrated) {
          const raw = localStorage.getItem("xuflow-projects");
          if (raw) {
            const count = await invoke<number>("db_migrate_from_localstorage", { frontendProjectsJson: raw });
            console.log("[settings] Migrated", count, "messages from localStorage to MySQL");
          }
        }
      } catch (e) {
        console.error("[settings] Migration check failed:", e);
      }
    } catch (e: any) {
      dbTestFail.value = true;
      dbConnected.value = false;
      console.error("[settings] DB connect failed:", e);
    } finally {
      dbConnecting.value = false;
    }
  }

  checkDbStatus();

// Push context window changes to Rust backend via Tauri
watch(
  () => [store.activeContextWindow, store.activeMinUserTurns] as const,
  async ([ctxWin, minTurns]) => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("set_context_window", { contextWindow: ctxWin });
      await invoke("set_min_user_turns", { minTurns });
    } catch (e) {
      // Silent — Tauri backend may not be available during SSR/setup
    }
  }
);
</script>

<template>
  <div class="settings-panel" :class="{ dark: themeStore.isDark }">
    <!-- 外观 -->
    <div v-if="props.activeSection === 'appearance'" class="section">
      <h2 class="section-title">外观</h2>
      <p class="section-desc">自定义 Xuflow 的主题、字体大小与对比度。</p>

      <!-- 主题 -->
      <NCard class="settings-card" style="margin-bottom: 20px">
        <template #header>
          <div class="card-header">
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
              <circle cx="10" cy="10" r="8" stroke="currentColor" stroke-width="1.6" />
              <path d="M10 2v4M10 14v4M2 10h4M14 10h4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
              <circle cx="10" cy="10" r="3" stroke="currentColor" stroke-width="1.6" />
            </svg>
            <span>主题</span>
          </div>
        </template>
        <div class="theme-cards">
          <div
            class="theme-card"
            :class="{ active: themeStore.variant === 'sunset' }"
            @click="themeStore.setVariant('sunset')"
          >
            <div class="theme-preview sunset-preview">
              <div class="preview-dot"></div>
              <div class="preview-line"></div>
            </div>
            <span class="theme-label">日落</span>
            <span class="theme-desc">深色暖调</span>
          </div>
          <div
            class="theme-card"
            :class="{ active: themeStore.variant === 'dawn' }"
            @click="themeStore.setVariant('dawn')"
          >
            <div class="theme-preview dawn-preview">
              <div class="preview-dot"></div>
              <div class="preview-line"></div>
            </div>
            <span class="theme-label">晨曦</span>
            <span class="theme-desc">浅色暖调</span>
          </div>
          <div
            class="theme-card"
            :class="{ active: themeStore.variant === 'system' }"
            @click="themeStore.setVariant('system')"
          >
            <div class="theme-preview system-preview">
              <div class="preview-dot"></div>
              <div class="preview-line"></div>
            </div>
            <span class="theme-label">系统</span>
            <span class="theme-desc">跟随系统</span>
          </div>
        </div>
      </NCard>

      <!-- UI 字体大小 -->
      <NCard class="settings-card" style="margin-bottom: 20px">
        <template #header>
          <div class="card-header">
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
              <path d="M4 6h12M4 10h8M4 14h4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
            </svg>
            <span>UI 字体大小</span>
          </div>
        </template>
        <div class="slider-section">
          <div class="slider-label-row">
            <span class="slider-desc">调整消息区域 AI 生成文本与用户提示词的大小</span>
            <span class="slider-value">{{ themeStore.uiFontSize }}px</span>
          </div>
          <NSlider
            :value="themeStore.uiFontSize"
            @update:value="themeStore.setUiFontSize($event)"
            :min="12"
            :max="20"
            :step="1"
            :marks="{ 12: '12', 14: '14', 16: '16', 18: '18', 20: '20' }"
          />
          <div class="preview-row">
            <div class="ui-preview-bubble user-preview" :style="{ fontSize: themeStore.uiFontSize + 'px' }">
              用户消息预览
            </div>
            <div class="ui-preview-bubble ai-preview" :style="{ fontSize: themeStore.uiFontSize + 'px' }">
              AI 回复文本预览
            </div>
          </div>
        </div>
      </NCard>

      <!-- 代码字体大小 -->
      <NCard class="settings-card" style="margin-bottom: 20px">
        <template #header>
          <div class="card-header">
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
              <path d="M6 5l-4 5 4 5M14 5l4 5-4 5M12 2l-4 16" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
            <span>代码字体大小</span>
          </div>
        </template>
        <div class="slider-section">
          <div class="slider-label-row">
            <span class="slider-desc">调整 Markdown 代码块中代码的字号</span>
            <span class="slider-value">{{ themeStore.codeFontSize }}px</span>
          </div>
          <NSlider
            :value="themeStore.codeFontSize"
            @update:value="themeStore.setCodeFontSize($event)"
            :min="11"
            :max="18"
            :step="1"
            :marks="{ 11: '11', 13: '13', 15: '15', 18: '18' }"
          />
          <div class="code-preview-block" :style="{ fontSize: themeStore.codeFontSize + 'px' }">
            <code>const greeting = "Hello, Xuflow!";</code>
          </div>
        </div>
      </NCard>

      <!-- 对比度 -->
      <NCard class="settings-card">
        <template #header>
          <div class="card-header">
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
              <circle cx="10" cy="10" r="8" stroke="currentColor" stroke-width="1.6" />
              <path d="M10 2a8 8 0 000 16V2z" fill="currentColor" />
            </svg>
            <span>对比度</span>
          </div>
        </template>
        <div class="slider-section">
          <div class="slider-label-row">
            <span class="slider-desc">调整界面文字与背景的对比强度</span>
            <span class="slider-value">{{ themeStore.contrast }}%</span>
          </div>
          <NSlider
            :value="themeStore.contrast"
            @update:value="themeStore.setContrast($event)"
            :min="80"
            :max="150"
            :step="5"
            :marks="{ 80: '80%', 100: '100%', 120: '120%', 150: '150%' }"
          />
          <div class="contrast-swatches">
            <div class="contrast-swatch" v-for="lvl in [80, 100, 120, 150]" :key="lvl"
              :class="{ active: themeStore.contrast === lvl }"
              :style="{ filter: `contrast(${themeStore.contrast}%)` }">
              <span class="swatch-text">Aa</span>
              <span class="swatch-label">{{ lvl }}%</span>
            </div>
          </div>
        </div>
      </NCard>
    </div>

    <!-- API 密钥 -->
    <div v-if="props.activeSection === 'api-keys'" class="section">
      <h2 class="section-title">API 密钥</h2>
      <p class="section-desc">管理你的 DeepSeek、Kimi 与火山引擎 API 密钥，以及当前使用的模型。</p>
      <NCard class="settings-card">
        <NForm label-placement="left" label-width="100">
          <NFormItem label="DeepSeek Key">
            <NInput v-model:value="store.deepseekApiKey" type="password" placeholder="输入 DeepSeek API Key" show-password-on="click" />
          </NFormItem>
          <NFormItem label="Kimi Key">
            <NInput v-model:value="store.kimiApiKey" type="password" placeholder="输入 Kimi (月之暗面) API Key" show-password-on="click" />
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

    <!-- 上下文管理 -->
    <div v-if="props.activeSection === 'context'" class="section">
      <h2 class="section-title">上下文管理</h2>
      <p class="section-desc">配置各模型的上下文窗口大小与 token 估算参数。默认值来自模型官方规格。</p>

      <!-- 上下文窗口大小 -->
      <NCard class="settings-card" style="margin-bottom: 20px">
        <template #header>
          <div class="card-header">
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
              <rect x="2" y="4" width="16" height="12" rx="2" stroke="currentColor" stroke-width="1.6"/>
              <path d="M6 8h8M6 11h5" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
            </svg>
            <span>上下文窗口大小</span>
          </div>
        </template>
        <div class="endpoint-grid">
          <NForm label-placement="top" size="small">
            <NFormItem
              v-for="m in allModelsForContext"
              :key="'ctx-' + m.value"
              :label="m.label"
            >
              <NInputNumber
                :value="store.getContextWindow(m.value)"
                @update:value="ctxWinHandler(m.value)"
                :min="1000"
                :max="1000000"
                :step="1000"
                :placeholder="String(m.contextWindow ?? 128000)"
                style="width: 100%"
              />
              <template #feedback>
                <span class="feedback-text">默认: {{ (m.contextWindow ?? 128000).toLocaleString() }} tokens</span>
              </template>
            </NFormItem>
          </NForm>
        </div>
      </NCard>

      <!-- 最小保留轮数 -->
      <NCard class="settings-card" style="margin-bottom: 20px">
        <template #header>
          <div class="card-header">
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
              <path d="M3 6h14M3 10h10M3 14h4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/>
            </svg>
            <span>最小保留轮数</span>
          </div>
        </template>
        <div class="slider-section">
          <div class="slider-label-row">
            <span class="slider-desc">
              上下文截断时保证保留的最后 N 轮对话（一轮 = 用户消息 + 完整的助手响应与工具调用）。
              增大此值可在长对话中保留更多上下文，但可能影响截断效果。
            </span>
            <span class="slider-value">{{ store.minUserTurns }} 轮</span>
          </div>
          <NSlider
            :value="store.minUserTurns"
            @update:value="store.setMinUserTurns($event)"
            :min="1"
            :max="10"
            :step="1"
            :marks="{ 1: '1', 3: '3', 5: '5', 7: '7', 10: '10' }"
          />
        </div>
      </NCard>

      <!-- 高级：Token 估算系数 -->
      <NCard class="settings-card">
        <template #header>
          <div class="card-header">
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
              <circle cx="10" cy="10" r="8" stroke="currentColor" stroke-width="1.6"/>
              <path d="M10 6v4l3 2" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
            </svg>
            <span>Token 估算系数（高级）</span>
          </div>
        </template>
        <p class="section-desc" style="margin-top: 0; margin-bottom: 12px;">
          调整字符到 token 的换算系数以提升估算精度。不同模型对中文/英文/代码的 token 化策略不同。
        </p>
        <div class="endpoint-grid">
          <NForm label-placement="top" size="small">
            <NFormItem
              v-for="m in allModelsForContext"
              :key="'coeff-' + m.value"
              :label="m.label"
            >
              <div style="display: flex; gap: 8px;">
                <NInputNumber
                  :value="getEstConfig(m.value).cjkCoeff"
                  @update:value="cjkHandler(m.value)"
                  :min="0.5"
                  :max="3.0"
                  :step="0.1"
                  placeholder="CJK"
                  style="flex: 1"
                >
                  <template #prefix>CJK</template>
                </NInputNumber>
                <NInputNumber
                  :value="getEstConfig(m.value).nonCjkCoeff"
                  @update:value="nonCjkHandler(m.value)"
                  :min="0.1"
                  :max="1.0"
                  :step="0.05"
                  placeholder="非CJK"
                  style="flex: 1"
                >
                  <template #prefix>非CJK</template>
                </NInputNumber>
                <NInputNumber
                  :value="getEstConfig(m.value).structuredCoeff"
                  @update:value="structHandler(m.value)"
                  :min="0.2"
                  :max="1.5"
                  :step="0.1"
                  placeholder="结构化"
                  style="flex: 1"
                >
                  <template #prefix>结构</template>
                </NInputNumber>
              </div>
            </NFormItem>
          </NForm>
        </div>
      </NCard>
    </div>


    <!-- 数据库 -->
    <div v-if="props.activeSection === 'database'" class="section">
      <h2 class="section-title">数据库</h2>
      <p class="section-desc">配置 MySQL 连接，替代默认的浏览器本地存储。数据和对话将持久化到指定数据库。</p>

      <!-- 连接配置 -->
      <NCard class="settings-card" style="margin-bottom: 20px">
        <template #header>
          <div class="card-header">
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
              <ellipse cx="10" cy="5" rx="8" ry="3" stroke="currentColor" stroke-width="1.6" />
              <path d="M2 5v10c0 1.66 3.58 3 8 3s8-1.34 8-3V5" stroke="currentColor" stroke-width="1.6" />
              <path d="M2 10c0 1.66 3.58 3 8 3s8-1.34 8-3" stroke="currentColor" stroke-width="1.2" />
            </svg>
            <span>MySQL 连接配置</span>
            <span class="db-status" :style="{ color: dbConnected ? '#22c55e' : '#9ca3af' }">
              {{ dbConnected ? '● 已连接' : '○ 未连接' }}
            </span>
          </div>
        </template>
        <NForm label-placement="left" label-width="80" size="small">
          <NFormItem label="主机地址">
            <NInput v-model:value="dbHost" placeholder="127.0.0.1" />
          </NFormItem>
          <NFormItem label="端口">
            <NInputNumber v-model:value="dbPort" :min="1" :max="65535" style="width: 100%" />
          </NFormItem>
          <NFormItem label="用户名">
            <NInput v-model:value="dbUser" placeholder="root" />
          </NFormItem>
          <NFormItem label="密码">
            <NInput v-model:value="dbPassword" type="password" placeholder="MySQL 密码" show-password-on="click" />
          </NFormItem>
          <NFormItem label="数据库名">
            <NInput v-model:value="dbName" placeholder="xuflow" />
          </NFormItem>
          <NFormItem>
            <div style="display: flex; gap: 8px;">
              <NButton @click="testConnection" :loading="dbConnecting" secondary>
                测试连接
              </NButton>
              <NButton @click="saveAndConnect" :loading="dbConnecting" type="primary">
                保存并连接
              </NButton>
              <NText v-if="dbTestOk" type="success" style="align-self: center; font-size: 13px;">✓ 连接成功</NText>
              <NText v-if="dbTestFail" type="error" style="align-self: center; font-size: 13px;">✗ 连接失败</NText>
            </div>
          </NFormItem>
        </NForm>
      </NCard>

      <!-- 使用说明 -->
      <NCard class="settings-card">
        <template #header>
          <div class="card-header">
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
              <circle cx="10" cy="10" r="8" stroke="currentColor" stroke-width="1.6" />
              <path d="M10 9v5M10 6v1" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
            </svg>
            <span>使用说明</span>
          </div>
        </template>
        <div class="about-content">
          <p>1. 确保 MySQL 服务已启动并创建了对应数据库。</p>
          <p>2. 执行建库 SQL：<code>CREATE DATABASE xuflow CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;</code></p>
          <p>3. 填写连接信息后点击「保存并连接」，系统自动创建所需表结构。</p>
          <p>4. 连接成功后，所有项目和会话数据将写入 MySQL，原有 localStorage 数据自动迁移。</p>
        </div>
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
/* 数据库状态指示 */
.db-status {
  font-size: 12px;
  font-weight: 500;
  margin-left: auto;
}
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

/* ── 外观 / Appearance ── */

/* 主题卡片选择 */
.theme-cards {
  display: flex;
  gap: 12px;
}

.theme-card {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 16px 12px;
  border: 2px solid rgba(0, 0, 0, 0.08);
  border-radius: 12px;
  cursor: pointer;
  transition: all 0.2s ease;
  background: #fafafa;
}

.theme-card:hover {
  border-color: #9ca3af;
  background: #f1f5f9;
  transform: translateY(-1px);
}

.theme-card.active {
  border-color: #6b7280;
  background: rgba(107, 114, 128, 0.06);
  box-shadow: 0 2px 8px rgba(107, 114, 128, 0.15);
}

.dark .theme-card {
  background: #1c1c22;
  border-color: rgba(255, 255, 255, 0.06);
}

.dark .theme-card:hover {
  border-color: #6b7280;
  background: #24242d;
}

.dark .theme-card.active {
  border-color: #9ca3af;
  background: rgba(156, 163, 175, 0.1);
}

/* 主题预览色块 */
.theme-preview {
  width: 48px;
  height: 32px;
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 3px;
  padding: 6px 8px;
  border: 1px solid rgba(0, 0, 0, 0.1);
}

.sunset-preview {
  background: linear-gradient(135deg, #2d2d38 0%, #3d3430 100%);
}

.sunset-preview .preview-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #f0a060;
}

.sunset-preview .preview-line {
  width: 24px;
  height: 2px;
  border-radius: 1px;
  background: rgba(240, 160, 96, 0.5);
}

.dawn-preview {
  background: linear-gradient(135deg, #fef7ed 0%, #fef3e2 100%);
}

.dawn-preview .preview-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #e89440;
}

.dawn-preview .preview-line {
  width: 24px;
  height: 2px;
  border-radius: 1px;
  background: rgba(232, 148, 64, 0.4);
}

.system-preview {
  background: linear-gradient(135deg, #f8fafc 0%, #1a1a20 100%);
}

.system-preview .preview-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #6b7280;
}

.system-preview .preview-line {
  width: 24px;
  height: 2px;
  border-radius: 1px;
  background: rgba(107, 114, 128, 0.5);
}

.theme-label {
  font-size: 14px;
  font-weight: 600;
  color: #1e293b;
}

.dark .theme-label {
  color: #e2e8f0;
}

.theme-desc {
  font-size: 11px;
  color: #94a3b8;
}

/* 滑块区域 */
.slider-section {
  padding: 4px 0;
}

.slider-label-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.slider-desc {
  font-size: 13px;
  color: #64748b;
}

.dark .slider-desc {
  color: #94a3b8;
}

.slider-value {
  font-size: 13px;
  font-weight: 600;
  color: #6b7280;
  background: rgba(107, 114, 128, 0.08);
  padding: 2px 10px;
  border-radius: 6px;
  min-width: 48px;
  text-align: center;
}

.dark .slider-value {
  color: #9ca3af;
  background: rgba(156, 163, 175, 0.12);
}

/* UI 字体预览气泡 */
.preview-row {
  display: flex;
  gap: 12px;
  margin-top: 16px;
}

.ui-preview-bubble {
  flex: 1;
  padding: 10px 14px;
  border-radius: 10px;
  line-height: 1.5;
}

.user-preview {
  background: linear-gradient(135deg, #6b7280, #4b5563);
  color: #fff;
  border-radius: 12px 12px 4px 12px;
  text-align: right;
}

.ai-preview {
  background: #f8fafc;
  border: 1px solid rgba(0, 0, 0, 0.08);
  color: #1e293b;
  border-radius: 12px 12px 12px 4px;
  text-align: left;
}

.dark .ai-preview {
  background: #1a1a20;
  border-color: rgba(255, 255, 255, 0.08);
  color: #e2e8f0;
}

/* 代码字体预览块 */
.code-preview-block {
  margin-top: 14px;
  background: #24242b;
  border-radius: 10px;
  padding: 12px 16px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  overflow-x: auto;
}

.code-preview-block code {
  color: #e2e8f0;
  font-family: "SF Mono", "Fira Code", monospace;
}

/* 对比度色块 */
.contrast-swatches {
  display: flex;
  gap: 8px;
  margin-top: 14px;
}

.contrast-swatch {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 10px;
  border-radius: 10px;
  background: #f8fafc;
  border: 2px solid rgba(0, 0, 0, 0.06);
  transition: border-color 0.2s ease;
}

.contrast-swatch.active {
  border-color: #6b7280;
}

.dark .contrast-swatch {
  background: #1c1c22;
  border-color: rgba(255, 255, 255, 0.06);
}

.dark .contrast-swatch.active {
  border-color: #9ca3af;
}

.swatch-text {
  font-size: 18px;
  font-weight: 600;
  color: #1e293b;
}

.dark .swatch-text {
  color: #e2e8f0;
}

.swatch-label {
  font-size: 11px;
  color: #94a3b8;
}
</style>
