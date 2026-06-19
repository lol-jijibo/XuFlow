import type { ToolCallEntry } from "../stores/project";

// ── Types ────────────────────────────────────────────────────────────

export type ToolCategory =
  | "file_read"
  | "file_write"
  | "search"
  | "directory"
  | "shell"
  | "web"
  | "git"
  | "plan";

export interface ToolGroup {
  category: ToolCategory;
  icon: string;
  label: string;
  entries: ToolCallEntry[];
  summary: string;
}

// ── Category mapping ─────────────────────────────────────────────────

const TOOL_CATEGORY_MAP: Record<string, ToolCategory> = {
  read_file: "file_read",
  write_file: "file_write",
  edit: "file_write",
  list_dir: "directory",
  grep: "search",
  glob: "search",
  bash: "shell",
  web_fetch: "web",
  git_status: "git",
  git_diff: "git",
  git_log: "git",
  git_add: "git",
  git_commit: "git",
  todo_write: "plan",
  propose_plan: "plan",
};

export const CATEGORY_META: Record<ToolCategory, { icon: string; label: string }> = {
  file_read: { icon: "📖", label: "文件读取" },
  file_write: { icon: "✏️", label: "文件编辑" },
  search: { icon: "🔍", label: "搜索" },
  directory: { icon: "📁", label: "目录" },
  shell: { icon: "💻", label: "命令" },
  web: { icon: "🌍", label: "网络" },
  git: { icon: "📊", label: "Git" },
  plan: { icon: "📋", label: "规划" },
};

/** Get the category for a tool name. */
export function getCategory(name: string): ToolCategory {
  return TOOL_CATEGORY_MAP[name] ?? "shell"; // fallback: shell (generic command)
}

// ── Grouping ─────────────────────────────────────────────────────────

/**
 * Group tool call entries by category, preserving the original order
 * (first occurrence of each category determines its position in the result).
 */
export function groupToolCalls(entries: ToolCallEntry[]): ToolGroup[] {
  const map = new Map<ToolCategory, ToolCallEntry[]>();

  for (const entry of entries) {
    const cat = getCategory(entry.name);
    if (!map.has(cat)) {
      map.set(cat, []);
    }
    map.get(cat)!.push(entry);
  }

  const groups: ToolGroup[] = [];
  for (const [category, groupEntries] of map) {
    const meta = CATEGORY_META[category];
    groups.push({
      category,
      icon: meta.icon,
      label: meta.label,
      entries: groupEntries,
      summary: summarizeGroup(category, groupEntries),
    });
  }

  return groups;
}

// ── Per-tool result summarization ────────────────────────────────────

/**
 * Extract a compact one-line summary from a tool call result.
 * Returns empty string if the tool hasn't completed yet.
 */
export function summarizeResult(entry: ToolCallEntry): string {
  if (!entry.resultDone) return "";

  const r = entry.result;

  // Tool still running (no result yet)
  if (r === undefined || r === null) return "";

  // Error / failure detection
  if (r.startsWith("Error:") || r.startsWith("Failed") || r.startsWith("Command blocked")) {
    return "失败";
  }

  switch (entry.name) {
    case "read_file":
      return summarizeReadFile(r);
    case "write_file":
      return summarizeWriteFile(r);
    case "edit":
      return summarizeEdit(r);
    case "list_dir":
      return summarizeListDir(r);
    case "grep":
      return summarizeGrep(r);
    case "glob":
      return summarizeGlob(r);
    case "bash":
      return summarizeBash(r);
    case "git_status":
      return summarizeGitStatus(r);
    case "git_diff":
      return summarizeGitDiff(r);
    case "git_log":
      return summarizeGitLog(r);
    case "git_add":
      return summarizeGitAdd(r);
    case "git_commit":
      return summarizeGitCommit(r);
    case "web_fetch":
      return summarizeWebFetch(r);
    case "todo_write":
      return summarizeTodoWrite(r);
    case "propose_plan":
      return summarizeProposePlan(r);
    default:
      return "完成";
  }
}

/** Generate a group-level summary by aggregating member summaries. */
export function summarizeGroup(category: ToolCategory, entries: ToolCallEntry[]): string {
  const done = entries.filter((e) => e.resultDone);
  if (done.length === 0) return "进行中…";

  // For single-entry groups, just use the individual summary
  if (entries.length === 1) {
    const s = summarizeResult(entries[0]);
    return s || "完成";
  }

  // Aggregate based on category
  switch (category) {
    case "file_read": {
      let totalLines = 0;
      for (const e of done) {
        const lines = countLines(e.result ?? "");
        if (lines > 0) totalLines += lines;
      }
      return totalLines > 0 ? `共 ${formatNumber(totalLines)} 行` : `${done.length} 个文件`;
    }
    case "file_write": {
      let totalBytes = 0;
      let edits = 0;
      for (const e of done) {
        const r = e.result ?? "";
        const byteMatch = r.match(/Successfully wrote (\d+) bytes/);
        if (byteMatch) totalBytes += parseInt(byteMatch[1], 10);
        const editMatch = r.match(/(\d+) replacement/);
        if (editMatch) edits += parseInt(editMatch[1], 10);
      }
      const parts: string[] = [];
      if (totalBytes > 0) parts.push(`${formatNumber(totalBytes)} 字节`);
      if (edits > 0) parts.push(`${edits} 处替换`);
      return parts.length > 0 ? parts.join("，") : `${done.length} 个文件`;
    }
    case "search": {
      let totalMatches = 0;
      let totalFiles = 0;
      for (const e of done) {
        const r = e.result ?? "";
        if (e.name === "grep") {
          const lines = r.split("\n").filter((l) => /^[^:]+:\d+:/.test(l)).length;
          totalMatches += lines;
        } else if (e.name === "glob") {
          const lines = r.split("\n").filter((l) => l.trim() && !l.startsWith("...")).length;
          // Don't count "No files found" as a file
          if (!r.startsWith("No files")) totalFiles += lines;
        }
      }
      const parts: string[] = [];
      if (totalMatches > 0) parts.push(`${totalMatches} 处匹配`);
      if (totalFiles > 0) parts.push(`${totalFiles} 个文件`);
      return parts.length > 0 ? parts.join("，") : "无结果";
    }
    case "directory": {
      let totalItems = 0;
      for (const e of done) {
        totalItems += e.result?.split("\n").filter((l) => l.trim()).length ?? 0;
      }
      return `${totalItems} 项`;
    }
    case "shell": {
      const successes = done.filter((e) => {
        const r = e.result ?? "";
        return r.includes("exit code: 0") || (!r.includes("exit code:") && !r.startsWith("Failed"));
      });
      const failures = done.filter((e) => {
        const r = e.result ?? "";
        return r.match(/exit code: [1-9]/) || r.startsWith("Failed");
      });
      const parts: string[] = [];
      if (successes.length > 0) parts.push(`${successes.length} 成功`);
      if (failures.length > 0) parts.push(`${failures.length} 失败`);
      return parts.length > 0 ? parts.join("，") : `${done.length} 完成`;
    }
    case "web": {
      let totalChars = 0;
      for (const e of done) {
        totalChars += (e.result ?? "").length;
      }
      return totalChars > 0 ? `${formatNumber(totalChars)} 字符` : `${done.length} 个请求`;
    }
    case "git": {
      const parts: string[] = [];
      let changes = 0;
      let commits = 0;
      let staged = 0;
      for (const e of done) {
        const r = e.result ?? "";
        if (e.name === "git_status") {
          if (!r.includes("clean")) {
            changes += r.split("\n").filter((l) => l.trim()).length;
          }
        } else if (e.name === "git_diff") {
          const plus = (r.match(/^\+[^+]/gm) || []).length;
          const minus = (r.match(/^-[^-]/gm) || []).length;
          if (plus > 0 || minus > 0) parts.push(`+${plus}/-${minus} 行`);
        } else if (e.name === "git_log") {
          commits += r.split("\n").filter((l) => l.trim()).length;
        } else if (e.name === "git_add") {
          const m = r.match(/Staged (\d+) file/);
          if (m) staged += parseInt(m[1], 10);
        }
      }
      if (changes > 0 && !parts.some((p) => p.includes("变更"))) parts.push(`${changes} 个变更`);
      if (commits > 0) parts.push(`${commits} 个提交`);
      if (staged > 0) parts.push(`暂存 ${staged} 个文件`);
      // Check for commits
      const hasCommit = done.some((e) => e.name === "git_commit" && (e.result ?? "").includes("Committed"));
      if (hasCommit) parts.push("已提交");
      return parts.length > 0 ? parts.join("，") : `${done.length} 完成`;
    }
    case "plan": {
      let totalTodos = 0;
      let totalSteps = 0;
      for (const e of done) {
        const r = e.result ?? "";
        if (e.name === "todo_write") {
          try {
            const parsed = JSON.parse(r);
            if (parsed.todos && Array.isArray(parsed.todos)) {
              totalTodos += parsed.todos.length;
            }
          } catch { /* not JSON, skip */ }
        } else if (e.name === "propose_plan") {
          try {
            const parsed = JSON.parse(r);
            if (parsed.steps && Array.isArray(parsed.steps)) {
              totalSteps += parsed.steps.length;
            }
          } catch { /* not JSON, skip */ }
        }
      }
      const parts: string[] = [];
      if (totalTodos > 0) parts.push(`${totalTodos} 项任务`);
      if (totalSteps > 0) parts.push(`${totalSteps} 个步骤`);
      return parts.length > 0 ? parts.join("，") : `${done.length} 完成`;
    }
    default:
      return `${done.length} 完成`;
  }
}

// ── Individual result parsers ────────────────────────────────────────

function summarizeReadFile(result: string): string {
  const lines = countLines(result);
  if (lines === 0) return "空文件";
  return `${lines} 行`;
}

function summarizeWriteFile(result: string): string {
  const byteMatch = result.match(/Successfully wrote (\d+) bytes/);
  if (byteMatch) {
    const bytes = parseInt(byteMatch[1], 10);
    return `${formatBytes(bytes)}`;
  }
  return "完成";
}

function summarizeEdit(result: string): string {
  const match = result.match(/(\d+) replacement/);
  if (match) return `${match[1]} 处替换`;
  return "完成";
}

function summarizeListDir(result: string): string {
  const lines = result.split("\n").filter((l) => l.trim()).length;
  return `${lines} 项`;
}

function summarizeGrep(result: string): string {
  if (result.startsWith("No matches found")) return "无匹配";
  const matchLines = result.split("\n").filter((l) => /^[^:]+:\d+:/.test(l)).length;
  if (matchLines === 0) return "无匹配";
  return `${matchLines} 处匹配`;
}

function summarizeGlob(result: string): string {
  if (result.startsWith("No files found")) return "无文件";
  const fileLines = result.split("\n").filter((l) => l.trim() && !l.startsWith("...")).length;
  return `${fileLines} 个文件`;
}

function summarizeBash(result: string): string {
  // Check for exit code
  const exitMatch = result.match(/exit code:\s*(\d+)/);
  if (exitMatch) {
    if (exitMatch[1] === "0") return "成功";
    return `退出码 ${exitMatch[1]}`;
  }
  // Count output lines
  const lines = countLines(result);
  if (lines > 0) return `${lines} 行输出`;
  return "成功";
}

function summarizeGitStatus(result: string): string {
  if (result.includes("clean")) return "干净";
  const lines = result.split("\n").filter((l) => l.trim()).length;
  return `${lines} 个变更`;
}

function summarizeGitDiff(result: string): string {
  if (result === "No changes." || !result.trim()) return "无变更";
  const plusCount = (result.match(/^\+[^+]/gm) || []).length;
  const minusCount = (result.match(/^-[^-]/gm) || []).length;
  return `+${plusCount}/-${minusCount} 行`;
}

function summarizeGitLog(result: string): string {
  if (result === "No commits yet." || !result.trim()) return "无提交";
  const lines = result.split("\n").filter((l) => l.trim()).length;
  return `${lines} 个提交`;
}

function summarizeGitAdd(result: string): string {
  const match = result.match(/Staged (\d+) file/);
  if (match) return `暂存 ${match[1]} 个文件`;
  return "完成";
}

function summarizeGitCommit(result: string): string {
  if (result.includes("Committed")) return "已提交";
  return "完成";
}

function summarizeWebFetch(result: string): string {
  if (result.startsWith("HTTP ")) return result.slice(0, 40); // e.g. "HTTP 404 Not Found"
  if (result.startsWith("Failed")) return "失败";
  const chars = result.length;
  if (chars > 0) return `${formatNumber(chars)} 字符`;
  return "完成";
}

function summarizeTodoWrite(result: string): string {
  try {
    const parsed = JSON.parse(result);
    if (parsed.todos && Array.isArray(parsed.todos)) {
      const total = parsed.todos.length;
      const completed = parsed.todos.filter(
        (t: { status: string }) => t.status === "completed"
      ).length;
      if (completed === total && total > 0) return `${total} 项 ✓`;
      return `${completed}/${total} 项`;
    }
  } catch {
    // not JSON
  }
  return "已更新";
}

function summarizeProposePlan(result: string): string {
  try {
    const parsed = JSON.parse(result);
    if (parsed.steps && Array.isArray(parsed.steps)) {
      return `${parsed.steps.length} 个步骤`;
    }
  } catch {
    // not JSON
  }
  return "已提出";
}

// ── Shared helpers ───────────────────────────────────────────────────

function countLines(s: string): number {
  return s.split("\n").filter((l) => l.trim() !== "").length;
}

function formatNumber(n: number): string {
  if (n >= 1000) {
    return (n / 1000).toFixed(n >= 10000 ? 0 : 1) + "k";
  }
  return n.toString();
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024) {
    return (bytes / (1024 * 1024)).toFixed(1) + " MB";
  }
  if (bytes >= 1024) {
    return (bytes / 1024).toFixed(1) + " KB";
  }
  return bytes + " 字节";
}

// ── Chip color helpers (for use in Vue components) ───────────────────

export const CATEGORY_COLORS: Record<ToolCategory, { bg: string; bgDark: string; text: string }> = {
  file_read: { bg: "rgba(59,130,246,0.1)", bgDark: "rgba(96,165,250,0.15)", text: "#3b82f6" },
  file_write: { bg: "rgba(249,115,22,0.1)", bgDark: "rgba(251,146,60,0.15)", text: "#f97316" },
  search: { bg: "rgba(34,197,94,0.1)", bgDark: "rgba(74,222,128,0.15)", text: "#22c55e" },
  directory: { bg: "rgba(168,85,247,0.1)", bgDark: "rgba(192,132,252,0.15)", text: "#a855f7" },
  shell: { bg: "rgba(107,114,128,0.1)", bgDark: "rgba(156,163,175,0.15)", text: "#6b7280" },
  web: { bg: "rgba(14,165,233,0.1)", bgDark: "rgba(56,189,248,0.15)", text: "#0ea5e9" },
  git: { bg: "rgba(236,72,153,0.1)", bgDark: "rgba(244,114,182,0.15)", text: "#ec4899" },
  plan: { bg: "rgba(234,179,8,0.1)", bgDark: "rgba(250,204,21,0.15)", text: "#eab308" },
};
