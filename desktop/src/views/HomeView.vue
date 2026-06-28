<script setup lang="ts">
import { ref, onUnmounted } from "vue";
import Sidebar from "../components/layout/Sidebar.vue";
import ChatPanel from "../components/chat/ChatPanel.vue";
import StatusBar from "../components/layout/StatusBar.vue";
import ReviewPanel from "../components/review/ReviewPanel.vue";
import ApprovalModal from "../components/approval/ApprovalModal.vue";
import { useThemeStore } from "../stores/theme";
import { useReviewStore } from "../stores/review";

const themeStore = useThemeStore();
const reviewStore = useReviewStore();

// 审查侧边栏可拖拽调整宽度，拖动对话区域与侧边栏之间的分隔线即可改变宽度
const reviewPanelWidth = ref(360);
const MIN_PANEL_WIDTH = 280;
const MAX_PANEL_WIDTH = 900;

const isDragging = ref(false);
let dragStartX = 0;
let dragStartWidth = 0;

/** 开始拖拽：记录初始鼠标位置和面板宽度，全局监听移动/释放事件，拖拽期间禁止选中文字 */
function onResizeStart(e: MouseEvent) {
  e.preventDefault();
  isDragging.value = true;
  dragStartX = e.clientX;
  dragStartWidth = reviewPanelWidth.value;
  document.body.style.userSelect = "none";
  document.body.style.cursor = "col-resize";
  document.addEventListener("mousemove", onResizeMove);
  document.addEventListener("mouseup", onResizeEnd);
}

/** 拖拽移动：根据鼠标水平位移计算新宽度，限制在最小值与最大值之间。
 *  向左拖动 = 面板变宽，向右拖动 = 面板变窄（分隔线左侧为对话区，右侧为审查面板）。 */
function onResizeMove(e: MouseEvent) {
  if (!isDragging.value) return;
  const deltaX = dragStartX - e.clientX;
  const newWidth = Math.min(MAX_PANEL_WIDTH, Math.max(MIN_PANEL_WIDTH, dragStartWidth + deltaX));
  reviewPanelWidth.value = newWidth;
}

/** 结束拖拽：移除全局事件监听，恢复文本选中与光标样式 */
function onResizeEnd() {
  isDragging.value = false;
  document.body.style.userSelect = "";
  document.body.style.cursor = "";
  document.removeEventListener("mousemove", onResizeMove);
  document.removeEventListener("mouseup", onResizeEnd);
}

onUnmounted(() => {
  document.body.style.userSelect = "";
  document.body.style.cursor = "";
  document.removeEventListener("mousemove", onResizeMove);
  document.removeEventListener("mouseup", onResizeEnd);
});
</script>

<template>
  <div class="home-root" :class="{ dark: themeStore.isDark }">
    <Sidebar />
    <div class="home-main">
      <ChatPanel />
      <StatusBar />
    </div>
    <!-- 拖拽分隔线：位于对话区域与审查侧边栏之间，鼠标悬浮显示可拖拽符号 -->
    <div
      v-if="reviewStore.visible"
      class="review-resize-handle"
      :class="{ dragging: isDragging, dark: themeStore.isDark }"
      @mousedown="onResizeStart"
    >
      <div class="resize-grip"></div>
    </div>
    <ReviewPanel :style="{ width: reviewPanelWidth + 'px', minWidth: reviewPanelWidth + 'px' }" />
    <ApprovalModal />
  </div>
</template>

<style scoped>
.home-root {
  display: flex;
  height: 100vh;
  overflow: hidden;
  background: #ffffff;
  transition: background-color 0.3s ease;
}

.home-root.dark {
  background: #101014;
}

.home-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

/* 审查面板拖拽分隔线：位于对话区域右侧，鼠标悬浮显示可拖拽符号与高亮 */
.review-resize-handle {
  position: relative;
  width: 6px;
  min-width: 6px;
  flex-shrink: 0;
  cursor: col-resize;
  background: transparent;
  transition: background-color 0.2s ease;
  z-index: 5;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* 拖拽热区扩大：实际可点击区域比视觉线条更宽，降低操作难度 */
.review-resize-handle::before {
  content: "";
  position: absolute;
  inset: 0 -4px;
  z-index: 0;
}

/* 悬浮时背景微亮，指示可拖拽 */
.review-resize-handle:hover,
.review-resize-handle.dragging {
  background: rgba(99, 102, 241, 0.08);
}

.review-resize-handle.dark:hover,
.review-resize-handle.dark.dragging {
  background: rgba(129, 140, 248, 0.12);
}

/* 中央拖拽指示条：细竖线 + 上下圆点，模拟拖拽手柄视觉效果 */
.resize-grip {
  width: 2px;
  height: 32px;
  border-radius: 1px;
  background: rgba(0, 0, 0, 0.12);
  transition: background-color 0.2s ease, height 0.2s ease;
  position: relative;
}

.dark .resize-grip {
  background: rgba(255, 255, 255, 0.1);
}

.review-resize-handle:hover .resize-grip,
.review-resize-handle.dragging .resize-grip {
  background: #6366f1;
  height: 48px;
}

.dark .review-resize-handle:hover .resize-grip,
.dark .review-resize-handle.dragging .resize-grip {
  background: #818cf8;
}

</style>
