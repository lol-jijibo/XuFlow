<script setup lang="ts">
import { computed } from "vue";
import { useAgentStore } from "../../stores/agent";
import { useThemeStore } from "../../stores/theme";
import { useReviewStore } from "../../stores/review";
import { useProjectStore } from "../../stores/project";

const agentStore = useAgentStore();
const themeStore = useThemeStore();
const reviewStore = useReviewStore();
const projectStore = useProjectStore();

/** 当前项目名：实时跟随用户在侧边栏的选择切换。 */
const currentProjectName = computed(() => projectStore.activeProject?.name ?? "Xuflow");

const statusText = computed(() => {
  return agentStore.isRunning ? "\u{1F504} 生成中..." : "⚡ 就绪";
});

const tokenLabel = computed(() => {
  const used = agentStore.tokenUsage;
  const max = agentStore.contextWindow;
  if (used <= 0 && max <= 0) return "";
  return `${formatK(used)} / ${formatK(max)}`;
});

/** Format a number as a compact K suffix (e.g. 128000 → "128K"). */
function formatK(n: number): string {
  if (n >= 1000) return `${(n / 1000).toFixed(0)}K`;
  return String(n);
}

const tokenLevel = computed(() => agentStore.tokenWarningLevel);

const appVersion = "v1.0.3";

/** 切换右侧审查面板，点击时自动拉取 diff */
async function toggleReview() {
  await reviewStore.togglePanel();
}
</script>

<template>
  <div class="status-bar" :class="{ dark: themeStore.isDark }">
    <div class="status-left">
      <span class="project-label">
        <svg width="15" height="15" viewBox="0 0 16 16" fill="none" class="project-icon-svg">
          <path d="M2 4.5A1.5 1.5 0 013.5 3h2.63a1.5 1.5 0 011.06.44l.77.77a1.5 1.5 0 001.06.44H12.5A1.5 1.5 0 0114 6.15V12.5a1.5 1.5 0 01-1.5 1.5h-9A1.5 1.5 0 012 12.5V4.5z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
        </svg>
        <span class="project-name">{{ currentProjectName }}</span>
      </span>
    </div>
    <div class="status-right">
      <!-- 审查面板切换按钮 -->
      <button
        class="review-toggle-btn"
        :class="{ active: reviewStore.visible }"
        @click="toggleReview"
        title="代码审查"
      >
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <path d="M2 3.5A1.5 1.5 0 013.5 2h2.63a1.5 1.5 0 011.06.44l.77.77a1.5 1.5 0 001.06.44H10.5A1.5 1.5 0 0112 5.15V10.5a1.5 1.5 0 01-1.5 1.5h-7A1.5 1.5 0 012 10.5V3.5z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>
        </svg>
        <span v-if="reviewStore.changedFileCount > 0" class="review-badge">{{ reviewStore.changedFileCount }}</span>
      </button>
      <span v-if="tokenLabel" class="token-label" :class="'token-' + tokenLevel">{{ tokenLabel }}</span>
      <span class="status-divider" v-if="tokenLabel">·</span>
      <span class="status-label">{{ statusText }}</span>
      <span class="version-label">{{ appVersion }}</span>
    </div>
  </div>
</template>

<style scoped>
.status-bar {
  height: 44px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 16px 0;
  flex-shrink: 0;
  background: #f3f4f6;
  transition: background-color 0.3s ease;
  user-select: none;
}

.status-bar.dark {
  background: #26272B;
}

/* ── Left: project status ─────────────────── */

.status-left {
  display: flex;
  align-items: center;
  flex-shrink: 0;
}

.project-label {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  font-size: 12.5px;
  font-weight: 500;
  color: #9ca3af;
  white-space: nowrap;
  letter-spacing: 0.01em;
}

.project-name {
  background: linear-gradient(105deg, #7dd3fc 0%, #99f6e4 48%, #d9f99d 100%);
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
  text-shadow: 0 0 18px rgba(125, 211, 252, 0.18);
}

.project-icon-svg {
  color: #7dd3fc;
  flex-shrink: 0;
}

.dark .project-icon-svg {
  color: #99f6e4;
}

.status-label {
  font-size: 12.5px;
  font-weight: 500;
  color: #6b7280;
  white-space: nowrap;
  letter-spacing: 0.01em;
}

.dark .status-label {
  color: #9ca3af;
}

.token-label {
  font-size: 12px;
  font-weight: 500;
  white-space: nowrap;
  font-family: "SF Mono", "Cascadia Code", "Fira Code", "JetBrains Mono", monospace;
}

.token-green  { color: #22c55e; }
.token-yellow { color: #eab308; }
.token-orange { color: #f59e0b; }
.token-red    { color: #ef4444; }

.status-divider {
  font-size: 12px;
  color: #9ca3af;
  margin: 0 4px;
  user-select: none;
}

.dark .status-divider {
  color: #6b7280;
}

/* ── Right: version ──────────────────────── */

.status-right {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-left: auto;
  flex-shrink: 0;
}

.version-label {
  font-size: 12px;
  font-weight: 500;
  color: #9ca3af;
  white-space: nowrap;
  letter-spacing: 0.02em;
  font-family: "SF Mono", "Cascadia Code", "Fira Code", "JetBrains Mono", monospace;
}

.dark .version-label {
  color: #6b7280;
}

/* ── 审查面板切换按钮 ── */
.review-toggle-btn {
  position: relative;
  width: 30px;
  height: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 7px;
  background: transparent;
  color: #9ca3af;
  cursor: pointer;
  transition: all 0.15s ease;
}

.review-toggle-btn:hover {
  background: rgba(0, 0, 0, 0.05);
  color: #6366f1;
}

.review-toggle-btn.active {
  background: rgba(99, 102, 241, 0.1);
  color: #6366f1;
}

.dark .review-toggle-btn {
  color: #6b7280;
}

.dark .review-toggle-btn:hover {
  background: rgba(255, 255, 255, 0.06);
  color: #818cf8;
}

.dark .review-toggle-btn.active {
  background: rgba(129, 140, 248, 0.12);
  color: #818cf8;
}

.review-badge {
  position: absolute;
  top: -2px;
  right: -2px;
  min-width: 14px;
  height: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 3px;
  border-radius: 7px;
  background: #6366f1;
  color: #fff;
  font-size: 9px;
  font-weight: 700;
  font-family: "SF Mono", "Cascadia Code", monospace;
}

.dark .review-badge {
  background: #818cf8;
}
</style>
