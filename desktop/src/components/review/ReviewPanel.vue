<script setup lang="ts">
import { computed } from "vue";
import { NScrollbar, NSpin } from "naive-ui";
import { useReviewStore } from "../../stores/review";
import { useThemeStore } from "../../stores/theme";
import ReviewToolbar from "./ReviewToolbar.vue";
import ReviewFileList from "./ReviewFileList.vue";
import ReviewDirTree from "./ReviewDirTree.vue";
import ReviewActions from "./ReviewActions.vue";

// 右侧代码审查侧边栏主容器，包含文件变更列表与批量操作栏

const store = useReviewStore();
const themeStore = useThemeStore();

const hasChanges = computed(() => store.diffFiles.length > 0);
</script>

<template>
  <Transition name="review-slide">
    <aside
      v-if="store.visible"
      class="review-panel"
      :class="{ dark: themeStore.isDark }"
    >
      <!-- 头部标题栏 -->
      <div class="review-header">
        <div class="review-header-left">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none" class="review-icon">
            <path d="M2 4.5A1.5 1.5 0 013.5 3h2.63a1.5 1.5 0 011.06.44l.77.77a1.5 1.5 0 001.06.44H12.5A1.5 1.5 0 0114 6.15V12.5a1.5 1.5 0 01-1.5 1.5h-9A1.5 1.5 0 012 12.5V4.5z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>
          </svg>
          <span class="review-title">代码审查</span>
          <span v-if="hasChanges" class="review-stats">
            <span class="review-stat review-stat--add">+{{ store.totalAdditions }}</span>
            <span class="review-stat review-stat--del">-{{ store.totalDeletions }}</span>
          </span>
        </div>
        <button class="review-close-btn" @click="store.closePanel()" title="关闭审查面板">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <path d="M3 3l8 8M11 3l-8 8" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/>
          </svg>
        </button>
      </div>

      <!-- 工具栏始终可见，保证用户可在任意状态下切换审查范围 -->
      <ReviewToolbar />

      <!-- 目录树：点击工具栏 📁 按钮切换显示，展示变更文件的目录结构 -->
      <ReviewDirTree v-if="store.showDirTree && hasChanges" :files="store.diffFiles" />

      <!-- 加载状态 -->
      <div v-if="store.loading" class="review-loading">
        <NSpin size="small" />
        <span class="review-loading-text">正在获取变更...</span>
      </div>

      <!-- 错误状态 -->
      <div v-else-if="store.error" class="review-error">
        <span class="review-error-text">{{ store.error }}</span>
        <button class="review-retry-btn" @click="store.fetchDiff()">重试</button>
      </div>

      <!-- 空状态 -->
      <div v-else-if="!hasChanges" class="review-empty">
        <svg width="32" height="32" viewBox="0 0 32 32" fill="none" class="review-empty-icon">
          <circle cx="16" cy="16" r="14" stroke="currentColor" stroke-width="1.5" opacity="0.3"/>
          <path d="M12 16l3 3 5-6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" opacity="0.4"/>
        </svg>
        <span class="review-empty-text">暂无代码变更</span>
        <span class="review-empty-hint">修改代码后刷新查看 diff</span>
      </div>

      <!-- 有变更时：文件列表 + 操作栏 -->
      <template v-else>
        <div class="review-body">
          <NScrollbar class="review-scroll">
            <ReviewFileList :files="store.diffFiles" />
          </NScrollbar>
        </div>
        <ReviewActions />
      </template>
    </aside>
  </Transition>
</template>

<style scoped>
/* ── 面板容器：右侧固定宽度，从右侧滑入 ── */
.review-panel {
  width: 320px;
  min-width: 320px;
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: #fafafa;
  border-left: 1px solid rgba(0, 0, 0, 0.06);
  transition: background-color 0.3s ease, border-color 0.3s ease;
  user-select: none;
  overflow: hidden;
}

.review-panel.dark {
  background: #18181b;
  border-left-color: rgba(255, 255, 255, 0.06);
}

/* ── 滑入/滑出过渡 ── */
.review-slide-enter-active {
  transition: all 0.25s cubic-bezier(0.25, 0.1, 0.25, 1);
}
.review-slide-leave-active {
  transition: all 0.2s cubic-bezier(0.25, 0.1, 0.25, 1);
}
.review-slide-enter-from {
  transform: translateX(40px);
  opacity: 0;
}
.review-slide-leave-to {
  transform: translateX(40px);
  opacity: 0;
}

/* ── 头部 ── */
.review-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px;
  flex-shrink: 0;
  border-bottom: 1px solid rgba(0, 0, 0, 0.05);
}

.dark .review-header {
  border-bottom-color: rgba(255, 255, 255, 0.05);
}

.review-header-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.review-icon {
  color: #6366f1;
  flex-shrink: 0;
}

.dark .review-icon {
  color: #818cf8;
}

.review-title {
  font-size: 13px;
  font-weight: 600;
  color: #1e293b;
  letter-spacing: -0.01em;
}

.dark .review-title {
  color: #e2e8f0;
}

/* 变更统计徽章组：新增绿色 + 删除红色，语义一目了然 */
.review-stats {
  display: flex;
  align-items: center;
  gap: 6px;
}

.review-stat {
  font-size: 14px;
  font-weight: 600;
  font-family: "Inter", "SF Pro Display", -apple-system, BlinkMacSystemFont, "Helvetica Neue", sans-serif;
  line-height: 1;
  letter-spacing: 0.02em;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

/* 新增数字：明亮绿色，高饱和醒目 */
.review-stat--add {
  color: #16cc5a;
}

/* 删除数字：明亮红色，高饱和醒目。减号视觉中心偏高，微调下移与加号对齐 */
.review-stat--del {
  color: #ff3b3b;
  position: relative;
  top: 0.5px;
}

.review-close-btn {
  width: 24px;
  height: 24px;
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

.review-close-btn:hover {
  background: rgba(0, 0, 0, 0.05);
  color: #374151;
}

.dark .review-close-btn {
  color: #6b7280;
}

.dark .review-close-btn:hover {
  background: rgba(255, 255, 255, 0.06);
  color: #d1d5db;
}

/* ── 加载状态 ── */
.review-loading {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
}

.review-loading-text {
  font-size: 12px;
  color: #9ca3af;
}

.dark .review-loading-text {
  color: #6b7280;
}

/* ── 错误状态 ── */
.review-error {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 20px;
}

.review-error-text {
  font-size: 12px;
  color: #ef4444;
  text-align: center;
}

.review-retry-btn {
  padding: 6px 16px;
  border: none;
  border-radius: 8px;
  background: rgba(99, 102, 241, 0.1);
  color: #6366f1;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.review-retry-btn:hover {
  background: rgba(99, 102, 241, 0.2);
}

/* ── 空状态 ── */
.review-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 20px;
}

.review-empty-icon {
  color: #9ca3af;
  margin-bottom: 4px;
}

.review-empty-text {
  font-size: 13px;
  font-weight: 500;
  color: #6b7280;
}

.dark .review-empty-text {
  color: #9ca3af;
}

.review-empty-hint {
  font-size: 11px;
  color: #c8c8c8;
}

.dark .review-empty-hint {
  color: #6b7280;
}

/* ── 主体内容区 ── */
.review-body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.review-scroll {
  flex: 1;
  min-height: 0;
}
</style>
