<script setup lang="ts">
import { computed, ref } from "vue";
import type { ToolCallEntry } from "../../stores/project";
import { summarizeResult } from "../../utils/toolSummary";

const props = defineProps<{
  entry: ToolCallEntry;
}>();

const collapsed = ref(true);

/** Human-friendly label for the tool call. */
const label = computed(() => {
  const a = props.entry.argsParsed;
  switch (props.entry.name) {
    case "read_file":
      return `📖 读取 ${fileName(a?.path)}`;
    case "write_file":
      return `✏️ 写入 ${fileName(a?.path)}`;
    case "edit":
      return `🔧 编辑 ${fileName(a?.path)}`;
    case "list_dir":
      return `📁 列出 ${shortPath(a?.path)}`;
    case "grep":
      return `🔍 搜索 "${a?.pattern ?? props.entry.arguments.slice(0, 60)}"`;
    case "glob":
      return `🌐 查找 "${a?.pattern ?? props.entry.arguments.slice(0, 60)}"`;
    case "bash":
      return `💻 ${(a?.command as string)?.slice(0, 80) ?? props.entry.arguments.slice(0, 80)}`;
    case "web_fetch":
      return `🌍 获取 ${(a?.url as string)?.slice(0, 60) ?? ""}`;
    case "todo_write":
      return `📋 更新任务`;
    case "propose_plan":
      return `📐 提出计划`;
    case "git_status":
      return `📊 git status`;
    case "git_diff":
      return `📊 git diff`;
    case "git_log":
      return `📜 git log`;
    case "git_add":
      return `➕ git add`;
    case "git_commit":
      return `✅ git commit`;
    default:
      return `🔨 ${props.entry.name}`;
  }
});

/** Smart result summary extracted from the tool output. */
const resultSummary = computed(() => {
  if (!props.entry.resultDone) return "";
  return summarizeResult(props.entry);
});

/** Full path as hover tooltip (for file-oriented tools). */
const fullPath = computed(() => {
  const p = props.entry.argsParsed?.path;
  return typeof p === "string" ? p : undefined;
});

/** Extract just the filename from a path (last segment only). */
function fileName(v: unknown): string {
  if (typeof v !== "string") return "";
  const parts = v.replace(/\\/g, "/").split("/");
  return parts[parts.length - 1] || v;
}

/** Show last 3 path segments for directory-like paths; use full path as title. */
function shortPath(v: unknown): string {
  if (typeof v !== "string") return "";
  const parts = v.replace(/\\/g, "/").split("/");
  if (parts.length > 3) return "…/" + parts.slice(-3).join("/");
  return v;
}

/** Truncate long results for display; 0 = no limit (show full). */
const MAX_PREVIEW = 5000;
const resultPreview = computed(() => {
  const r = props.entry.result ?? "";
  if (r.length <= MAX_PREVIEW) return r;
  return r.slice(0, MAX_PREVIEW) + `\n… (${r.length - MAX_PREVIEW} 更多字符)`;
});

const resultLines = computed(() => resultPreview.value.split("\n").length);
</script>

<template>
  <div class="tool-card" :class="{ collapsed, 'has-result': !!entry.result }">
    <!-- Header — always visible -->
    <div class="tool-header" @click="collapsed = !collapsed" :title="fullPath">
      <span class="tool-chevron">{{ collapsed ? "▸" : "▾" }}</span>
      <span class="tool-label">{{ label }}</span>
      <span v-if="resultSummary" class="tool-result-summary">— {{ resultSummary }}</span>
      <span v-if="!entry.resultDone && !entry.result" class="tool-spinner">⏳</span>
      <span v-else-if="entry.resultDone" class="tool-check">✓</span>
    </div>

    <!-- Expandable body -->
    <div v-if="!collapsed" class="tool-body">
      <!-- Arguments (compact, dim) -->
      <div class="tool-section">
        <div class="tool-section-title">参数</div>
        <pre class="tool-json">{{
          JSON.stringify(entry.argsParsed ?? entry.arguments, null, 2)
        }}</pre>
      </div>

      <!-- Result -->
      <div v-if="entry.result" class="tool-section">
        <div class="tool-section-title">
          结果
          <span class="tool-meta">· {{ resultLines }} 行</span>
        </div>
        <!-- File content: show with line numbers -->
        <pre v-if="entry.name === 'read_file'" class="tool-code">{{
          resultPreview
        }}</pre>
        <!-- List dir / glob: show as compact list -->
        <pre v-else-if="entry.name === 'list_dir' || entry.name === 'glob'" class="tool-code">{{
          resultPreview
        }}</pre>
        <!-- Default: monospace block -->
        <pre v-else class="tool-code">{{ resultPreview }}</pre>
      </div>
    </div>
  </div>
</template>

<style scoped>
.tool-card {
  margin: 6px 0;
  border: 1px solid rgba(128, 128, 128, 0.2);
  border-radius: 8px;
  font-size: 13px;
  overflow: hidden;
  transition: border-color 0.15s;
}
.tool-card:hover {
  border-color: rgba(128, 128, 128, 0.4);
}
.tool-card.has-result {
  border-left: 3px solid var(--n-primary-color, #6366f1);
}

/* Header */
.tool-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  cursor: pointer;
  user-select: none;
  background: rgba(128, 128, 128, 0.04);
}
.tool-header:hover {
  background: rgba(128, 128, 128, 0.08);
}
.tool-chevron {
  font-size: 11px;
  color: #888;
  width: 14px;
  flex-shrink: 0;
}
.tool-label {
  flex: 1;
  font-weight: 500;
  color: var(--n-text-color, #e4e4e7);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.tool-spinner {
  font-size: 12px;
  animation: spin 1s linear infinite;
}
.tool-check {
  color: #22c55e;
  font-size: 12px;
}

.tool-result-summary {
  font-size: 11px;
  color: #888;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 180px;
  flex-shrink: 1;
}

/* Body */
.tool-body {
  padding: 0 12px 10px;
}
.tool-section {
  margin-top: 8px;
}
.tool-section-title {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  color: #888;
  margin-bottom: 4px;
}
.tool-meta {
  font-weight: 400;
  color: #666;
}
.tool-json,
.tool-code {
  margin: 0;
  padding: 8px 10px;
  font-size: 12px;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
  background: rgba(128, 128, 128, 0.06);
  border: 1px solid rgba(128, 128, 128, 0.1);
  border-radius: 6px;
  max-height: 400px;
  overflow-y: auto;
  font-family: "SF Mono", "Cascadia Code", "Fira Code", monospace;
}
.tool-code {
  /* For read_file output with line numbers */
  tab-size: 4;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
