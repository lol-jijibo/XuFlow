<script setup lang="ts">
import { ref, watch } from "vue";
import type { ToolGroup } from "../../utils/toolSummary";
import { useThemeStore } from "../../stores/theme";
import ToolCallCard from "./ToolCallCard.vue";

const props = defineProps<{
  group: ToolGroup;
  expanded?: boolean;
}>();

const themeStore = useThemeStore();
const collapsed = ref(!props.expanded);

// Sync with parent when expanded prop changes externally
watch(
  () => props.expanded,
  (val) => {
    collapsed.value = !val;
  }
);
</script>

<template>
  <div class="tool-group" :class="{ collapsed, dark: themeStore.isDark }">
    <!-- Group header — always visible, click to toggle -->
    <div class="group-header" @click="collapsed = !collapsed">
      <span class="group-chevron">{{ collapsed ? "▸" : "▾" }}</span>
      <span class="group-label">
        {{ group.icon }} {{ group.label }}
        <span class="group-count">({{ group.entries.length }})</span>
      </span>
      <span v-if="group.summary" class="group-summary">— {{ group.summary }}</span>
    </div>

    <!-- Expanded card list -->
    <div v-if="!collapsed" class="group-body">
      <ToolCallCard
        v-for="tc in group.entries"
        :key="tc.id"
        :entry="tc"
      />
    </div>
  </div>
</template>

<style scoped>
.tool-group {
  margin: 4px 0;
  border-radius: 8px;
  overflow: hidden;
}

/* Group header — matches ToolCallCard header style */
.group-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  cursor: pointer;
  user-select: none;
  background: rgba(128, 128, 128, 0.03);
  border-radius: 6px;
  transition: background 0.12s ease;
}

.group-header:hover {
  background: rgba(128, 128, 128, 0.06);
}

.group-chevron {
  font-size: 11px;
  color: #888;
  width: 14px;
  flex-shrink: 0;
}

.group-label {
  font-size: 13px;
  font-weight: 500;
  color: var(--n-text-color, #e4e4e7);
  white-space: nowrap;
}

.group-count {
  font-weight: 400;
  color: #888;
  margin-left: 2px;
}

.group-summary {
  font-size: 11px;
  color: #777;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex: 1;
}

/* Body — left border indent to show hierarchy */
.group-body {
  margin-top: 4px;
  padding-left: 16px;
  border-left: 2px solid rgba(128, 128, 128, 0.15);
}

/* ── Dark mode ── */
.dark .group-header {
  background: rgba(128, 128, 128, 0.04);
}

.dark .group-header:hover {
  background: rgba(128, 128, 128, 0.08);
}

.dark .group-label {
  color: #e4e4e7;
}

.dark .group-summary {
  color: #999;
}
</style>
