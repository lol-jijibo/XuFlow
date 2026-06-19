<script setup lang="ts">
import { useAgentStore } from "../../stores/agent";
import { useThemeStore } from "../../stores/theme";

const store = useAgentStore();
const themeStore = useThemeStore();

async function approvePlan() {
  store.pendingPlan = null;
  await store.respondApproval(true);
}

async function rejectPlan() {
  store.pendingPlan = null;
  await store.respondApproval(false);
}
</script>

<template>
  <div
    v-if="store.pendingPlan"
    class="plan-card"
    :class="{ dark: themeStore.isDark }"
  >
    <div class="plan-header">
      <div class="plan-icon">
        <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
          <path d="M3 5h14M3 10h10M3 15h14" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/>
        </svg>
      </div>
      <h3 class="plan-title">{{ store.pendingPlan.title }}</h3>
    </div>

    <div class="plan-body">
      <!-- Steps -->
      <div class="plan-section">
        <h4 class="plan-section-title">实施步骤</h4>
        <ol class="plan-steps">
          <li
            v-for="(step, idx) in store.pendingPlan.steps"
            :key="idx"
            class="plan-step-item"
          >
            {{ step }}
          </li>
        </ol>
      </div>

      <!-- Files -->
      <div class="plan-section">
        <h4 class="plan-section-title">涉及文件</h4>
        <div class="plan-files">
          <code
            v-for="(file, idx) in store.pendingPlan.files_to_modify"
            :key="idx"
            class="plan-file-tag"
          >
            {{ file }}
          </code>
        </div>
      </div>
    </div>

    <div class="plan-actions">
      <button class="plan-btn plan-btn-reject" @click="rejectPlan">
        拒绝 — 修改计划
      </button>
      <button class="plan-btn plan-btn-approve" @click="approvePlan">
        批准 — 开始执行
      </button>
    </div>
  </div>
</template>

<style scoped>
.plan-card {
  margin: 0 44px 24px;
  background: rgba(255, 255, 255, 0.72);
  backdrop-filter: blur(16px);
  border: 1px solid rgba(99, 102, 241, 0.15);
  border-radius: 16px;
  padding: 24px;
  transition: background 0.3s ease, border-color 0.3s ease;
}

.plan-card.dark {
  background: rgba(30, 30, 38, 0.82);
  border-color: rgba(129, 140, 248, 0.15);
}

.plan-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 20px;
}

.plan-icon {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  background: rgba(99, 102, 241, 0.08);
  color: #6366f1;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.dark .plan-icon {
  background: rgba(129, 140, 248, 0.12);
  color: #a5b4fc;
}

.plan-title {
  font-size: 17px;
  font-weight: 650;
  color: #1e293b;
  margin: 0;
}

.dark .plan-title {
  color: #e2e8f0;
}

.plan-body {
  margin-bottom: 20px;
}

.plan-section {
  margin-bottom: 16px;
}

.plan-section:last-child {
  margin-bottom: 0;
}

.plan-section-title {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: #94a3b8;
  margin: 0 0 8px;
}

.dark .plan-section-title {
  color: #64748b;
}

.plan-steps {
  margin: 0;
  padding-left: 20px;
}

.plan-step-item {
  font-size: 13.5px;
  line-height: 1.7;
  color: #475569;
  padding: 2px 0;
}

.dark .plan-step-item {
  color: #cbd5e1;
}

.plan-files {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.plan-file-tag {
  font-size: 12px;
  font-family: monospace;
  background: rgba(0, 0, 0, 0.04);
  padding: 3px 8px;
  border-radius: 6px;
  color: #6366f1;
}

.dark .plan-file-tag {
  background: rgba(255, 255, 255, 0.05);
  color: #a5b4fc;
}

.plan-actions {
  display: flex;
  gap: 10px;
  justify-content: flex-end;
}

.plan-btn {
  padding: 10px 20px;
  border-radius: 10px;
  font-size: 13.5px;
  font-weight: 600;
  border: none;
  cursor: pointer;
  transition: all 0.2s ease;
}

.plan-btn-reject {
  background: rgba(0, 0, 0, 0.05);
  color: #64748b;
}

.plan-btn-reject:hover {
  background: rgba(239, 68, 68, 0.08);
  color: #ef4444;
}

.dark .plan-btn-reject {
  background: rgba(255, 255, 255, 0.06);
  color: #94a3b8;
}

.dark .plan-btn-reject:hover {
  background: rgba(248, 113, 113, 0.12);
  color: #f87171;
}

.plan-btn-approve {
  background: #6366f1;
  color: #fff;
}

.plan-btn-approve:hover {
  background: #4f46e5;
  box-shadow: 0 2px 12px rgba(99, 102, 241, 0.3);
}

.dark .plan-btn-approve {
  background: #818cf8;
}

.dark .plan-btn-approve:hover {
  background: #6366f1;
}
</style>
