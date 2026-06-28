import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { DiffFile, DiffScope, ReviewComment } from "../utils/diffParser";
import { parseDiff, findDiffFile } from "../utils/diffParser";

// 审查侧边栏全局状态：diff 数据、面板可见性、git 操作、评论管理

export const useReviewStore = defineStore("review", () => {
  // ── 面板可见性 ──
  const visible = ref(false);
  /** 已变更文件数量（用于 StatusBar 徽章显示） */
  const changedFileCount = ref(0);

  // ── Diff 数据 ──
  const scope = ref<DiffScope>("uncommitted");
  const diffFiles = ref<DiffFile[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  /** 全部文件折叠/展开 */
  const collapseAll = ref(false);
  /** 目录树展示开关 */
  const showDirTree = ref(false);
  /** 当前导航定位的文件路径 */
  const activeFilePath = ref<string | null>(null);

  function toggleCollapseAll() {
    collapseAll.value = !collapseAll.value;
  }

  function toggleDirTree() {
    showDirTree.value = !showDirTree.value;
  }

  function setActiveFile(path: string | null) {
    activeFilePath.value = path;
  }

  /** 在系统文件管理器中定位文件 */
  async function revealInExplorer(filePath: string) {
    const root = await resolveProjectRoot();
    if (!root) return;
    const absolutePath = root.replace(/\\/g, "/") + "/" + filePath;
    try {
      await invoke("reveal_in_explorer", { path: absolutePath });
    } catch (e) {
      console.error("[review] revealInExplorer error:", e);
      error.value = String(e);
    }
  }

  /** 在系统文件管理器中打开项目根目录 */
  async function openProjectDir() {
    const root = await resolveProjectRoot();
    if (!root) return;
    try {
      await invoke("reveal_in_explorer", { path: root });
    } catch (e) {
      console.error("[review] openProjectDir error:", e);
      error.value = String(e);
    }
  }

  /** 获取项目根目录：优先用 store 中的 path，fallback 到 Rust 工作目录 */
  async function resolveProjectRoot(): Promise<string | null> {
    const { useProjectStore } = await import("./project");
    const projectStore = useProjectStore();
    if (projectStore.activeProject?.path) {
      return projectStore.activeProject.path;
    }
    try {
      return await invoke<string>("get_working_dir");
    } catch {
      error.value = "无法获取项目根目录";
      return null;
    }
  }

  /** 按状态分组的文件列表 */
  const stagedFiles = computed(() => diffFiles.value.filter((f) => f.status !== "deleted"));
  const deletedFiles = computed(() => diffFiles.value.filter((f) => f.status === "deleted"));

  /** 总变更统计 */
  const totalAdditions = computed(() => diffFiles.value.reduce((s, f) => s + f.additions, 0));
  const totalDeletions = computed(() => diffFiles.value.reduce((s, f) => s + f.deletions, 0));

  // ── 暂存状态（追踪哪些文件/行被暂存） ──
  const stagedPaths = ref<Set<string>>(new Set());

  function isPathStaged(path: string): boolean {
    return stagedPaths.value.has(path);
  }

  function toggleStaged(path: string) {
    const s = new Set(stagedPaths.value);
    if (s.has(path)) s.delete(path);
    else s.add(path);
    stagedPaths.value = s;
  }

  // ── AI 审查结果 ──
  const reviewing = ref(false);

  // ── 获取 diff 数据 ──
  async function fetchDiff(scopeOverride?: DiffScope) {
    const targetScope = scopeOverride || scope.value;
    loading.value = true;
    error.value = null;

    try {
      let rawDiff = "";

      if (targetScope === "uncommitted") {
        // 获取未暂存 + 已暂存的变更
        const unstaged = await invoke<string>("git_diff_raw", {
          args: "",
        }).catch(() => "");
        const staged = await invoke<string>("git_diff_raw", {
          args: "--cached",
        }).catch(() => "");
        rawDiff = [unstaged, staged].filter(Boolean).join("\n");
      } else if (targetScope === "branch") {
        rawDiff = await invoke<string>("git_diff_raw", {
          args: "origin/main...HEAD",
        }).catch(() => invoke<string>("git_diff_raw", { args: "main...HEAD" }).catch(() => ""));
      } else if (targetScope === "last-turn") {
        // last-turn 由前端追踪 Agent 写入的文件列表来过滤
        // 先获取所有未提交的 diff，后续在 UI 中过滤
        rawDiff = await invoke<string>("git_diff_raw", {
          args: "",
        }).catch(() => "");
      }

      diffFiles.value = parseDiff(rawDiff);
      changedFileCount.value = diffFiles.value.length;
    } catch (e) {
      console.error("[review] fetchDiff error:", e);
      error.value = String(e);
      diffFiles.value = [];
      changedFileCount.value = 0;
    } finally {
      loading.value = false;
    }
  }

  /** 切换面板可见性，打开时自动拉取 diff */
  async function togglePanel() {
    visible.value = !visible.value;
    if (visible.value) {
      await fetchDiff();
    }
  }

  /** 打开面板并指定范围 */
  async function openPanel(s?: DiffScope) {
    if (s) scope.value = s;
    visible.value = true;
    await fetchDiff(s);
  }

  function closePanel() {
    visible.value = false;
  }

  // ── Git 操作 ──

  /** 暂存单个文件 */
  async function stageFile(path: string) {
    try {
      await invoke("git_add", { files: path });
      stagedPaths.value = new Set([...stagedPaths.value, path]);
      // 暂存后刷新 diff
      await fetchDiff();
    } catch (e) {
      console.error("[review] stageFile error:", e);
    }
  }

  /** 取消暂存单个文件 */
  async function unstageFile(path: string) {
    try {
      await invoke("git_reset_file", { path });
      const s = new Set(stagedPaths.value);
      s.delete(path);
      stagedPaths.value = s;
      await fetchDiff();
    } catch (e) {
      console.error("[review] unstageFile error:", e);
    }
  }

  /** 回退单个文件的变更 */
  async function revertFile(path: string) {
    try {
      await invoke("git_checkout_file", { path });
      await fetchDiff();
    } catch (e) {
      console.error("[review] revertFile error:", e);
    }
  }

  /** 暂存所有变更 */
  async function stageAll() {
    try {
      await invoke("git_add", { files: "." });
      stagedPaths.value = new Set(diffFiles.value.map((f) => f.path));
      await fetchDiff();
    } catch (e) {
      console.error("[review] stageAll error:", e);
    }
  }

  /** 回退所有变更 */
  async function revertAll() {
    try {
      await invoke("git_checkout_all");
      stagedPaths.value = new Set();
      await fetchDiff();
    } catch (e) {
      console.error("[review] revertAll error:", e);
    }
  }

  // ── AI 审查 ──

  /** 将 diff 上下文作为消息发送给 Agent，触发 AI 审查 */
  function buildReviewPrompt(focusAreas: string[] = ["bugs", "security", "perf", "style"]): string {
    const focusText = focusAreas.join("、");
    const diffSummary = diffFiles.value
      .map((f) => `- ${f.path} (+${f.additions}/-${f.deletions})`)
      .join("\n");

    return `请审查以下代码变更，重点关注：${focusText}。

变更文件：
${diffSummary}

请逐文件分析，对每个发现的问题标明：文件路径、行号、严重程度（error/warning/info）、分类（bug/security/perf/style）、问题描述和修复建议。`;
  }

  // ── 评论管理 ──

  /** 给指定文件的指定行添加评论 */
  function addComment(
    filePath: string,
    lineContent: string,
    comment: Omit<ReviewComment, "id" | "timestamp">
  ) {
    const file = findDiffFile(diffFiles.value, filePath);
    if (!file) return;

    for (const hunk of file.hunks) {
      for (const line of hunk.lines) {
        if (line.content === lineContent) {
          line.comments.push({
            ...comment,
            id: `comment-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
            timestamp: Date.now(),
          });
          return;
        }
      }
    }
  }

  return {
    // 面板状态
    visible,
    changedFileCount,
    // diff 数据
    scope,
    diffFiles,
    stagedFiles,
    deletedFiles,
    totalAdditions,
    totalDeletions,
    loading,
    error,
    // 折叠 & 文件导航
    collapseAll,
    toggleCollapseAll,
    showDirTree,
    toggleDirTree,
    activeFilePath,
    setActiveFile,
    // 暂存
    stagedPaths,
    isPathStaged,
    toggleStaged,
    // 审查
    reviewing,
    // 操作
    fetchDiff,
    togglePanel,
    openPanel,
    closePanel,
    stageFile,
    unstageFile,
    revertFile,
    stageAll,
    revertAll,
    buildReviewPrompt,
    addComment,
    revealInExplorer,
    openProjectDir,
  };
});
