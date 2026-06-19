<script setup lang="ts">
import { computed } from "vue";
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

// Parse edit tool params for diff display
const isEdit = computed(() => store.pendingApproval?.tool === "edit");
const editOld = computed(() => {
  if (!isEdit.value) return "";
  try { return JSON.parse(store.pendingApproval!.params).old_string || ""; } catch { return ""; }
});
const editNew = computed(() => {
  if (!isEdit.value) return "";
  try { return JSON.parse(store.pendingApproval!.params).new_string || ""; } catch { return ""; }
});
const editFile = computed(() => {
  if (!store.pendingApproval) return "";
  try { return JSON.parse(store.pendingApproval.params).path || ""; } catch { return ""; }
});
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
        <button class="approval-close" @click="reject" title="关闭">
          <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
            <path d="M5 5l8 8M13 5l-8 8" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
          </svg>
        </button>
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
        <!-- Edit diff preview -->
        <div v-if="isEdit" class="approval-field">
          <NText class="field-label">文件</NText>
          <div class="field-value">{{ editFile }}</div>
        </div>
        <div v-if="isEdit" class="approval-field">
          <NText class="field-label">变更预览</NText>
          <div class="edit-diff">
            <div class="edit-diff-removed">
              <span class="edit-diff-marker">−</span>
              <pre class="edit-diff-text">{{ editOld }}</pre>
            </div>
            <div class="edit-diff-added">
              <span class="edit-diff-marker">+</span>
              <pre class="edit-diff-text">{{ editNew }}</pre>
            </div>
          </div>
        </div>
        <!-- Raw params for non-edit tools -->
        <div v-else class="approval-field">
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
  overflow: visible;
}

.approval-header {
  text-align: center;
  margin-bottom: 24px;
  position: relative;
}

.approval-close {
  position: absolute;
  top: 0;
  right: 0;
  width: 32px;
  height: 32px;
  border-radius: 8px;
  border: none;
  background: transparent;
  color: #94a3b8;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: color 0.15s ease, background-color 0.15s ease;
}

.approval-close:hover {
  color: #ef4444;
  background: rgba(239, 68, 68, 0.08);
}

.dark .approval-close {
  color: #64748b;
}

.dark .approval-close:hover {
  color: #f87171;
  background: rgba(248, 113, 113, 0.12);
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
  color: #6b7280;
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
  background: #1c1c22;
  border-color: rgba(255, 255, 255, 0.08);
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

/* ── Edit diff preview ──────────────────────── */
.edit-diff {
  border-radius: 10px;
  overflow: hidden;
  border: 1px solid rgba(0, 0, 0, 0.08);
}

.dark .edit-diff {
  border-color: rgba(255, 255, 255, 0.08);
}

.edit-diff-removed {
  background: rgba(239, 68, 68, 0.08);
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 8px 12px;
}

.edit-diff-added {
  background: rgba(34, 197, 94, 0.08);
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 8px 12px;
}

.edit-diff-marker {
  font-family: monospace;
  font-size: 12px;
  font-weight: 700;
  flex-shrink: 0;
  width: 14px;
}

.edit-diff-removed .edit-diff-marker {
  color: #ef4444;
}

.edit-diff-added .edit-diff-marker {
  color: #22c55e;
}

.edit-diff-text {
  margin: 0;
  font-size: 12px;
  font-family: monospace;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-all;
  flex: 1;
  color: #475569;
}

.dark .edit-diff-text {
  color: #cbd5e1;
}
</style>
