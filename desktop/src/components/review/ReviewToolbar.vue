<script setup lang="ts">
import { useReviewStore } from "../../stores/review";
import { useThemeStore } from "../../stores/theme";
import type { DiffScope } from "../../utils/diffParser";

// 审查范围选择器：未提交变更 / 分支变更 / 最近一轮

const store = useReviewStore();
const themeStore = useThemeStore();

const scopeOptions: { value: DiffScope; label: string }[] = [
  { value: "uncommitted", label: "未提交变更" },
  { value: "branch", label: "分支变更" },
  { value: "last-turn", label: "最近一轮" },
];

async function changeScope(s: DiffScope) {
  if (s === store.scope) return;
  await store.fetchDiff(s);
  store.scope = s;
}
</script>

<template>
  <div class="review-toolbar" :class="{ dark: themeStore.isDark }">
    <div class="toolbar-scope">
      <button
        v-for="opt in scopeOptions"
        :key="opt.value"
        class="scope-btn"
        :class="{ active: store.scope === opt.value }"
        @click="changeScope(opt.value)"
      >
        {{ opt.label }}
      </button>
    </div>
    <button
      class="toolbar-refresh"
      @click="store.fetchDiff()"
      title="刷新变更"
    >
      <svg width="13" height="13" viewBox="0 0 13 13" fill="none">
        <path d="M2.5 6.5a4 4 0 018 0" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
        <path d="M10.5 6.5a4 4 0 01-8 0" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
        <path d="M10.5 3v3.5H7" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
        <path d="M2.5 10V6.5H6" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </button>
  </div>
</template>

<style scoped>
.review-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.04);
  flex-shrink: 0;
}

.dark .review-toolbar {
  border-bottom-color: rgba(255, 255, 255, 0.04);
}

/* ── 范围选择按钮组 ── */
.toolbar-scope {
  display: flex;
  gap: 2px;
  background: rgba(0, 0, 0, 0.04);
  border-radius: 7px;
  padding: 2px;
}

.dark .toolbar-scope {
  background: rgba(255, 255, 255, 0.04);
}

.scope-btn {
  padding: 3px 8px;
  border: none;
  border-radius: 5px;
  background: transparent;
  color: #6b7280;
  font-size: 11px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
  white-space: nowrap;
}

.scope-btn:hover {
  color: #374151;
}

.scope-btn.active {
  background: #fff;
  color: #1f2937;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.06);
}

.dark .scope-btn {
  color: #9ca3af;
}

.dark .scope-btn:hover {
  color: #d1d5db;
}

.dark .scope-btn.active {
  background: #2C2C2E;
  color: #e5e7eb;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
}

/* ── 刷新按钮 ── */
.toolbar-refresh {
  width: 26px;
  height: 26px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: #9ca3af;
  cursor: pointer;
  transition: all 0.15s ease;
}

.toolbar-refresh:hover {
  background: rgba(0, 0, 0, 0.05);
  color: #6366f1;
}

.dark .toolbar-refresh {
  color: #6b7280;
}

.dark .toolbar-refresh:hover {
  background: rgba(255, 255, 255, 0.06);
  color: #818cf8;
}
</style>
