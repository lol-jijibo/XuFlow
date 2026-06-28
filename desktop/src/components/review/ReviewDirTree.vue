<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Icon } from "@iconify/vue";
import type { DiffFile } from "../../utils/diffParser";
import { useReviewStore } from "../../stores/review";

// 从 diff 文件列表构建项目目录树，在审查面板内展示目录结构
// 默认全部展开，用户可手动折叠

interface DirNode {
  name: string;
  files: DiffFile[];
  children: Record<string, DirNode>;
}

const props = defineProps<{
  files: DiffFile[];
}>();

const store = useReviewStore();

function buildTree(files: DiffFile[]): DirNode {
  const root: DirNode = { name: "", files: [], children: {} };
  for (const f of files) {
    const parts = f.path.split("/");
    let cur = root;
    for (let i = 0; i < parts.length; i++) {
      if (i === parts.length - 1) {
        cur.files.push(f);
      } else {
        if (!cur.children[parts[i]]) {
          cur.children[parts[i]] = { name: parts[i], files: [], children: {} };
        }
        cur = cur.children[parts[i]];
      }
    }
  }
  return root;
}

const tree = computed(() => buildTree(props.files));

/** 收集目录树中所有目录的路径 */
function collectDirPaths(node: DirNode, parentPath: string): string[] {
  const paths: string[] = [];
  for (const [name, child] of Object.entries(node.children)) {
    const dirPath = parentPath ? parentPath + "/" + name : name;
    paths.push(dirPath);
    paths.push(...collectDirPaths(child, dirPath));
  }
  return paths;
}

// ── 展开状态：默认全部展开 ──
const expanded = ref<Set<string>>(new Set());

/** 目录树打开时或文件列表变化时自动展开全部目录 */
watch(
  [() => store.showDirTree, () => props.files],
  ([isOpen]) => {
    if (isOpen) {
      expanded.value = new Set(collectDirPaths(tree.value, ""));
    }
  },
  { immediate: true }
);

function toggleExpand(dirPath: string) {
  const next = new Set(expanded.value);
  if (next.has(dirPath)) next.delete(dirPath);
  else next.add(dirPath);
  expanded.value = next;
}

// ── 文件导航 ──
function navigate(filePath: string) {
  store.setActiveFile(filePath);
  requestAnimationFrame(() => {
    const el = document.getElementById("review-file-" + filePath.replace(/[^a-zA-Z0-9_-]/g, "_"));
    if (el) {
      el.scrollIntoView({ behavior: "smooth", block: "center" });
      el.classList.add("file-flash");
      setTimeout(() => el.classList.remove("file-flash"), 1200);
    }
  });
}

// ── 目录变更统计（仅统计直接文件，不含子目录；避免中间层目录重复展示相同数量）──
function nodeStats(node: DirNode): { adds: number; dels: number } {
  let adds = 0, dels = 0;
  for (const f of node.files) { adds += f.additions; dels += f.deletions; }
  // 不再递归累加子目录，子目录数量由各自节点独立展示
  return { adds, dels };
}

function sortedDirs(node: DirNode): [string, DirNode][] {
  return Object.entries(node.children).sort(([a], [b]) => a.localeCompare(b));
}

function sortedFiles(node: DirNode): DiffFile[] {
  return [...node.files].sort((a, b) => a.path.localeCompare(b.path));
}

// ── 语言 Iconify 图标映射（与 ReviewFileList 保持一致）──
const EXT_ICON_MAP: Record<string, string> = {
  ts: "logos:typescript-icon", tsx: "logos:typescript-icon",
  js: "logos:javascript", jsx: "logos:javascript", mjs: "logos:javascript", cjs: "logos:javascript",
  vue: "logos:vue", html: "logos:html-5", htm: "logos:html-5",
  css: "logos:css-3", scss: "logos:sass", less: "logos:less",
  rs: "logos:rust", go: "logos:go", py: "logos:python",
  java: "logos:java", kt: "logos:kotlin", cs: "logos:c-sharp",
  php: "logos:php", rb: "logos:ruby", swift: "logos:swift",
  c: "logos:c", h: "logos:c", cpp: "logos:c-plusplus", hpp: "logos:c-plusplus",
  json: "logos:json", yaml: "logos:yaml", yml: "logos:yaml", toml: "logos:toml",
  xml: "logos:xml", svg: "logos:svg", sql: "logos:postgresql",
  graphql: "logos:graphql", gql: "logos:graphql", proto: "logos:protobuf",
  sh: "logos:bash-icon", bash: "logos:bash-icon", zsh: "logos:bash-icon",
  md: "logos:markdown", mdx: "logos:markdown",
  dockerfile: "logos:docker-icon", svelte: "logos:svelte-icon",
  ex: "logos:elixir", exs: "logos:elixir", scala: "logos:scala",
  clj: "logos:clojure", cljs: "logos:clojure", hs: "logos:haskell", erl: "logos:erlang",
};

/** 根据文件路径返回 Iconify 图标名，未匹配返回 null */
function langIcon(filePath: string): string | null {
  const ext = filePath.split(".").pop()?.toLowerCase() || "";
  const base = filePath.split("/").pop()?.toLowerCase() || "";
  if (EXT_ICON_MAP[base]) return EXT_ICON_MAP[base];
  if (EXT_ICON_MAP[ext]) return EXT_ICON_MAP[ext];
  return null;
}

/** 未匹配图标时的兜底文本标签 */
function langFallback(filePath: string): string {
  const ext = filePath.split(".").pop()?.toLowerCase() || "";
  return ext.slice(0, 3).toUpperCase() || "?";
}
</script>

<template>
  <div class="dir-tree" v-if="files.length > 0">
    <div class="dir-tree-title">项目目录</div>
    <!-- 渲染根级内容：子目录 -->
    <DirNodeView
      v-for="[name, child] in sortedDirs(tree)"
      :key="'d-' + name"
      :node="child"
      :dirPath="name"
      :indent="0"
      :expanded="expanded"
      @toggle="toggleExpand"
      @navigate="navigate"
      :nodeStats="nodeStats"
      :sortedDirs="sortedDirs"
      :sortedFiles="sortedFiles"
      :langIcon="langIcon"
      :langFallback="langFallback"
    />
    <!-- 根级文件 -->
    <div
      v-for="f in sortedFiles(tree)"
      :key="'rf-' + f.path"
      class="dt-file"
      @click.stop="navigate(f.path)"
    >
      <Icon
        v-if="langIcon(f.path)"
        :icon="langIcon(f.path) ?? ''"
        class="dt-file-icon"
        width="13"
        height="14"
      />
      <span v-else class="dt-file-fallback">{{ langFallback(f.path) }}</span>
      <span class="dt-file-name">{{ f.path.split("/").pop() || f.path }}</span>
      <span class="dt-stats">
        <span class="st-add">+{{ f.additions }}</span>
        <span class="st-del">-{{ f.deletions }}</span>
      </span>
    </div>
  </div>
</template>

<!-- ═══════════════════════════════════════════
  递归目录节点子组件（同一文件内 defineComponent）
  ═══════════════════════════════════════════ -->
<script lang="ts">
import { defineComponent, h, type PropType } from "vue";

// 在第二 script 块中重新声明 DirNode 类型（与 setup 块共享同一运行时）
interface DirNodeLocal {
  name: string;
  files: import("../../utils/diffParser").DiffFile[];
  children: Record<string, DirNodeLocal>;
}

const DirNodeView = defineComponent({
  name: "DirNodeView",
  props: {
    node: { type: Object as PropType<DirNodeLocal>, required: true },
    dirPath: { type: String, required: true },
    indent: { type: Number, default: 0 },
    expanded: { type: Object as PropType<Set<string>>, required: true },
    nodeStats: { type: Function as PropType<(n: DirNodeLocal) => { adds: number; dels: number }>, required: true },
    sortedDirs: { type: Function as PropType<(n: DirNodeLocal) => [string, DirNodeLocal][]>, required: true },
    sortedFiles: { type: Function as PropType<(n: DirNodeLocal) => DiffFile[]>, required: true },
    langIcon: { type: Function as PropType<(p: string) => string | null>, required: true },
    langFallback: { type: Function as PropType<(p: string) => string>, required: true },
  },
  emits: ["toggle", "navigate"],
  setup(props, { emit }) {
    const isOpen = () => (props.expanded as Set<string>).has(props.dirPath);
    const stats = () => (props.nodeStats as Function)(props.node) as { adds: number; dels: number };
    const dirs = () => (props.sortedDirs as Function)(props.node) as [string, DirNodeLocal][];
    const files = () => (props.sortedFiles as Function)(props.node) as import("../../utils/diffParser").DiffFile[];
    const padLeft = () => (12 + props.indent * 16) + "px";

    return () => {
      const children: any[] = [];
      const isOpenVal = isOpen();
      const s = stats();
      const indentPx = padLeft();

      // 目录行
      children.push(
        h("div", {
          class: "dt-dir",
          style: { paddingLeft: indentPx },
          onClick: () => emit("toggle", props.dirPath),
          key: "dir",
        }, [
          h("svg", {
            class: ["dt-chevron", { rotated: isOpenVal }],
            width: 10, height: 10, viewBox: "0 0 10 10", fill: "none",
            innerHTML: '<path d="M3.5 2L6.5 5L3.5 8" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>',
          }),
          h("svg", {
            class: "dt-dir-icon",
            width: 14, height: 14, viewBox: "0 0 14 14", fill: "none",
            innerHTML: isOpenVal
              ? '<path d="M2 3.5A1.5 1.5 0 013.5 2h2.1a1.5 1.5 0 011.06.44l.68.68a1.5 1.5 0 001.06.44H11.5A1.5 1.5 0 0113 5.06V11.5a1.5 1.5 0 01-1.5 1.5h-9A1.5 1.5 0 012 11.5V3.5z" stroke="currentColor" stroke-width="1.15" fill="currentColor" fill-opacity="0.1" stroke-linejoin="round"/><line x1="4.5" y1="8" x2="9.5" y2="8" stroke="currentColor" stroke-width="1" stroke-linecap="round" opacity="0.5"/>'
              : '<path d="M2 3.5A1.5 1.5 0 013.5 2h2.1a1.5 1.5 0 011.06.44l.68.68a1.5 1.5 0 001.06.44H11.5A1.5 1.5 0 0113 5.06V11.5a1.5 1.5 0 01-1.5 1.5h-9A1.5 1.5 0 012 11.5V3.5z" stroke="currentColor" stroke-width="1.15" stroke-linejoin="round"/>',
          }),
          h("span", { class: "dt-dir-name" }, props.node.name),
          // 仅当目录直接包含变更文件时才展示数量，避免中间层目录与子目录重复展示相同数量
          props.node.files.length > 0
            ? h("span", { class: "dt-stats" }, [
                h("span", { class: "st-add" }, "+" + s.adds),
                h("span", { class: "st-del" }, "-" + s.dels),
              ])
            : null,
        ])
      );

      // 展开时渲染子内容
      if (isOpenVal) {
        // 子目录
        for (const [name, child] of dirs()) {
          children.push(
            h(DirNodeView, {
              key: "d-" + name,
              node: child,
              dirPath: props.dirPath + "/" + name,
              indent: props.indent + 1,
              expanded: props.expanded,
              nodeStats: props.nodeStats,
              sortedDirs: props.sortedDirs,
              sortedFiles: props.sortedFiles,
              langIcon: props.langIcon,
              langFallback: props.langFallback,
              onToggle: (p: string) => emit("toggle", p),
              onNavigate: (p: string) => emit("navigate", p),
            })
          );
        }
        // 子文件：优先使用 Iconify 图标，未匹配用兜底文本
        for (const f of files()) {
          const iconName = (props.langIcon as Function)(f.path) as string | null;
          const fallback = (props.langFallback as Function)(f.path) as string;
          children.push(
            h("div", {
              class: "dt-file",
              style: { paddingLeft: (12 + (props.indent + 1) * 16) + "px" },
              key: "f-" + f.path,
              onClick: (e: MouseEvent) => { e.stopPropagation(); emit("navigate", f.path); },
            }, [
              iconName
                ? h(Icon, { icon: iconName, class: "dt-file-icon", width: "13", height: "14" })
                : h("span", { class: "dt-file-fallback" }, fallback),
              h("span", { class: "dt-file-name" }, f.path.split("/").pop() || f.path),
              h("span", { class: "dt-stats" }, [
                h("span", { class: "st-add" }, "+" + f.additions),
                h("span", { class: "st-del" }, "-" + f.deletions),
              ]),
            ])
          );
        }
      }

      return h("div", { class: "dt-node" }, children);
    };
  },
});

export { DirNodeView };
</script>

<style>
/* 目录树全局样式 — DirNodeView 通过 render 函数渲染，不受父组件 scoped 限制 */
.dir-tree {
  padding: 4px 0 2px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.05);
}

.dark .dir-tree {
  border-bottom-color: rgba(255, 255, 255, 0.05);
}

.dir-tree-title {
  padding: 4px 12px 6px;
  font-size: 10.5px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: #94a3b8;
  user-select: none;
}

.dark .dir-tree-title {
  color: #9ca3af;
}

/* ── 目录行（scoped 样式会通过 data-v-xxx 穿透 render 函数生成的元素） ── */
.dt-dir {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px 4px 12px;
  cursor: pointer;
  transition: background 0.1s ease;
}

.dt-dir:hover {
  background: rgba(0, 0, 0, 0.025);
}

.dark .dt-dir:hover {
  background: rgba(255, 255, 255, 0.025);
}

.dt-chevron {
  flex-shrink: 0;
  color: #9ca3af;
  transition: transform 0.15s ease;
}

.dt-chevron.rotated {
  transform: rotate(90deg);
}

.dark .dt-chevron {
  color: #6b7280;
}

.dt-dir-icon {
  flex-shrink: 0;
  color: #9ca3af;
}

.dark .dt-dir-icon {
  color: #6b7280;
}

.dt-dir-name {
  flex: 1;
  font-size: 12px;
  font-weight: 500;
  color: #374151;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}

.dark .dt-dir-name {
  color: #d1d5db;
}

/* ── 文件行 ── */
.dt-file {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 3px 10px 3px 26px;
  cursor: pointer;
  transition: background 0.1s ease;
}

.dt-file:hover {
  background: rgba(99, 102, 241, 0.05);
}

.dark .dt-file:hover {
  background: rgba(129, 140, 248, 0.06);
}

/* 语言产品图标 */
.dt-file-icon {
  flex-shrink: 0;
  border-radius: 2px;
}

/* 未匹配图标时的兜底文本标签 */
.dt-file-fallback {
  min-width: 18px;
  height: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 2px;
  font-size: 8px;
  font-weight: 600;
  font-family: "Inter", "SF Pro Display", -apple-system, sans-serif;
  letter-spacing: 0.02em;
  flex-shrink: 0;
  padding: 0 2px;
  color: #9ca3af;
  background: rgba(156, 163, 175, 0.1);
}

.dt-file-name {
  flex: 1;
  font-size: 11.5px;
  font-weight: 450;
  color: #4b5563;
  font-family: "SF Mono", "Cascadia Code", "Fira Code", monospace;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}

.dark .dt-file-name {
  color: #c4c4c8;
}

/* ── 变更统计 ── */
.dt-stats {
  display: flex;
  gap: 4px;
  font-size: 11px;
  font-family: "SF Mono", "Cascadia Code", monospace;
  flex-shrink: 0;
}

.st-add { color: #16a34a; }
.st-del { color: #dc2626; }

.dark .st-add { color: #4ade80; }
.dark .st-del { color: #f87171; }
</style>
