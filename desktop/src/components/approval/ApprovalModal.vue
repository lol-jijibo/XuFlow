<script setup lang="ts">
import { NModal, NCard, NButton, NText } from "naive-ui";
import { useAgentStore } from "../../stores/agent";
import { useThemeStore } from "../../stores/theme";

const store = useAgentStore();
const themeStore = useThemeStore();

async function approve() {
  await store.respondApproval(true);
}

async function reject() {
  await store.respondApproval(false);
}
</script>

<template>
  <NModal
    :show="!!store.pendingApproval"
    :mask-closable="false"
    :bordered="false"
    class="approval-modal"
  >
    <NCard
      class="approval-card"
      :class="{ dark: themeStore.isDark }"
      :bordered="false"
    >
      <div class="approval-header">
        <div class="approval-icon warning">
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
            <path
              d="M12 9v4M12 17h.01M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </div>
        <h3 class="approval-title">工具执行审批</h3>
        <p class="approval-desc">AI 请求执行以下工具操作，请确认是否允许：</p>
      </div>

      <div v-if="store.pendingApproval" class="approval-content">
        <div class="approval-field">
          <NText class="field-label">工具名称</NText>
          <div class="field-value tool-name">
            {{ store.pendingApproval.tool }}
          </div>
        </div>
        <div class="approval-field">
          <NText class="field-label">执行参数</NText>
          <pre class="approval-params">{{ store.pendingApproval.params }}</pre>
        </div>
      </div>

      <div class="approval-actions">
        <NButton size="large" @click="reject" class="action-btn reject-btn">
          拒绝
        </NButton>
        <NButton
          type="primary"
          size="large"
          @click="approve"
          class="action-btn approve-btn"
        >
          批准执行
        </NButton>
      </div>
    </NCard>
  </NModal>
</template>

<style scoped>
.approval-card {
  width: 480px;
  border-radius: 16px;
  overflow: hidden;
}

.approval-header {
  text-align: center;
  margin-bottom: 24px;
}

.approval-icon {
  width: 56px;
  height: 56px;
  border-radius: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 auto 16px;
}

.approval-icon.warning {
  background: rgba(245, 158, 11, 0.1);
  color: #f59e0b;
}

.approval-title {
  font-size: 20px;
  font-weight: 700;
  margin: 0 0 8px;
  color: #1e293b;
}

.dark .approval-title {
  color: #e2e8f0;
}

.approval-desc {
  font-size: 14px;
  color: #64748b;
  margin: 0;
}

.approval-content {
  margin-bottom: 24px;
}

.approval-field {
  margin-bottom: 16px;
}

.field-label {
  display: block;
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: #94a3b8;
  margin-bottom: 6px;
}

.field-value {
  font-size: 14px;
  color: #475569;
}

.dark .field-value {
  color: #94a3b8;
}

.tool-name {
  font-family: monospace;
  font-weight: 600;
  color: #6366f1;
}

.approval-params {
  background: #f8fafc;
  padding: 12px;
  border-radius: 8px;
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 200px;
  overflow-y: auto;
  margin: 0;
  font-family: monospace;
  line-height: 1.5;
  border: 1px solid rgba(0, 0, 0, 0.06);
}

.dark .approval-params {
  background: #1e1e3f;
  border-color: rgba(255, 255, 255, 0.06);
}

.approval-actions {
  display: flex;
  gap: 12px;
}

.action-btn {
  flex: 1;
  height: 44px;
  font-weight: 600;
}

.reject-btn {
  border-radius: 10px;
}

.approve-btn {
  border-radius: 10px;
}
</style>
