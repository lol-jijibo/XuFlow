import hljs from "highlight.js";

// 解析 git diff 输出为结构化数据，支持按文件扩展名检测语言并进行语法高亮

/** 单行 diff 类型 */
export type DiffLineType = "add" | "remove" | "context" | "header";

/** 单行 diff 数据 */
export interface DiffLine {
  type: DiffLineType;
  oldLineNo?: number;
  newLineNo?: number;
  content: string;
  /** 行级评论列表，由审查面板添加 */
  comments: ReviewComment[];
}

/** 单个人工/AI 审查评论 */
export interface ReviewComment {
  id: string;
  author: "user" | "agent";
  content: string;
  /** AI 审查结果中附带的严重程度 */
  severity?: "error" | "warning" | "info";
  /** AI 审查结果中附带的分类 */
  category?: "bug" | "security" | "perf" | "style";
  /** AI 建议的修复代码 */
  suggestion?: string;
  timestamp: number;
}

/** 一个 diff hunk（@@ 头部 + 行列表） */
export interface DiffHunk {
  header: string;
  lines: DiffLine[];
}

/** 一个文件的完整 diff 信息 */
export interface DiffFile {
  path: string;
  status: "added" | "modified" | "deleted" | "renamed";
  oldPath?: string; // 重命名时的旧路径
  additions: number;
  deletions: number;
  hunks: DiffHunk[];
  /** 文件扩展名对应的 highlight.js 语言标识 */
  language: string;
}

/** diff 查看范围 */
export type DiffScope = "uncommitted" | "branch" | "last-turn";

/** 文件扩展名 -> highlight.js 语言标识映射表，覆盖 80+ 主流开发语言 */
const EXTENSION_LANG_MAP: Record<string, string> = {
  // 前端
  ".ts": "typescript",
  ".tsx": "typescript",
  ".js": "javascript",
  ".jsx": "javascript",
  ".mjs": "javascript",
  ".cjs": "javascript",
  ".vue": "xml",        // Vue SFC 用 xml 模式高亮
  ".svelte": "xml",
  ".html": "xml",
  ".htm": "xml",
  ".css": "css",
  ".scss": "scss",
  ".less": "less",
  // 后端
  ".rs": "rust",
  ".go": "go",
  ".py": "python",
  ".pyi": "python",
  ".pyx": "python",
  ".java": "java",
  ".kt": "kotlin",
  ".kts": "kotlin",
  ".cs": "csharp",
  ".php": "php",
  ".rb": "ruby",
  ".ex": "elixir",
  ".exs": "elixir",
  ".erl": "erlang",
  ".hrl": "erlang",
  ".hs": "haskell",
  ".scala": "scala",
  ".sc": "scala",
  ".clj": "clojure",
  ".cljs": "clojure",
  ".edn": "clojure",
  ".swift": "swift",
  // 系统
  ".c": "c",
  ".h": "c",
  ".cpp": "cpp",
  ".cxx": "cpp",
  ".cc": "cpp",
  ".hpp": "cpp",
  ".zig": "zig",
  ".asm": "x86asm",
  ".s": "x86asm",
  ".S": "x86asm",
  // 数据 / 配置
  ".json": "json",
  ".yaml": "yaml",
  ".yml": "yaml",
  ".toml": "ini",
  ".xml": "xml",
  ".svg": "xml",
  ".sql": "sql",
  ".graphql": "graphql",
  ".gql": "graphql",
  ".proto": "protobuf",
  // 脚本
  ".sh": "bash",
  ".bash": "bash",
  ".zsh": "bash",
  ".fish": "bash",
  ".ps1": "powershell",
  ".psm1": "powershell",
  ".psd1": "powershell",
  ".lua": "lua",
  ".pl": "perl",
  ".pm": "perl",
  // 配置 / 构建
  ".dockerfile": "dockerfile",
  ".mk": "makefile",
  ".cmake": "cmake",
  ".nginx": "nginx",
  ".hcl": "hcl",
  ".tf": "hcl",
  ".tfvars": "hcl",
  // 文档
  ".md": "markdown",
  ".mdx": "markdown",
  ".tex": "latex",
  ".rst": "python", // reStructuredText
  // 其他
  ".env": "bash",
  ".gitignore": "bash",
  ".lock": "json",
};

/** 从文件路径检测语言标识 */
export function detectLanguage(filePath: string): string {
  // 特殊文件名匹配
  const basename = filePath.split("/").pop() || filePath;
  const lowerBasename = basename.toLowerCase();

  if (lowerBasename === "dockerfile") return "dockerfile";
  if (lowerBasename === "makefile") return "makefile";
  if (lowerBasename === "cmakelists.txt") return "cmake";

  // 扩展名匹配
  const dotIdx = basename.lastIndexOf(".");
  if (dotIdx >= 0) {
    const ext = basename.substring(dotIdx).toLowerCase();
    if (EXTENSION_LANG_MAP[ext]) return EXTENSION_LANG_MAP[ext];
    // 复合扩展名: .d.ts → typescript
    if (ext === ".d" && basename.endsWith(".d.ts")) return "typescript";
  }

  // fallback: 纯文本
  return "plaintext";
}

/** 对单行代码进行语法高亮，返回 HTML 片段 */
export function highlightLine(code: string, language: string): string {
  if (!code.trim()) return escapeHtml(code);

  try {
    if (language === "plaintext" || !hljs.getLanguage(language)) {
      return escapeHtml(code);
    }
    const result = hljs.highlight(code, { language });
    return result.value;
  } catch {
    return escapeHtml(code);
  }
}

/** HTML 转义，防止 XSS */
export function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

/** 解析 unified diff 文本为结构化数据 */
export function parseDiff(rawDiff: string): DiffFile[] {
  if (!rawDiff.trim()) return [];

  const files: DiffFile[] = [];
  const lines = rawDiff.split("\n");

  let currentFile: DiffFile | null = null;
  let currentHunk: DiffHunk | null = null;
  let additions = 0;
  let deletions = 0;

  function flushFile() {
    if (currentFile) {
      if (currentHunk && currentHunk.lines.length > 0) {
        currentFile.hunks.push(currentHunk);
      }
      currentFile.additions = additions;
      currentFile.deletions = deletions;
      files.push(currentFile);
    }
    currentFile = null;
    currentHunk = null;
    additions = 0;
    deletions = 0;
  }

  for (const line of lines) {
    // 文件头: diff --git a/path b/path
    if (line.startsWith("diff --git ")) {
      flushFile();
      const match = line.match(/diff --git a\/(.+) b\/(.+)/);
      if (match) {
        currentFile = {
          path: match[2] || match[1],
          status: "modified",
          additions: 0,
          deletions: 0,
          hunks: [],
          language: detectLanguage(match[2] || match[1]),
        };
      }
      continue;
    }

    if (!currentFile) continue;

    // 新旧文件模式 / 索引行 / 文件类型变更 — 跳过
    if (
      line.startsWith("old mode ") ||
      line.startsWith("new mode ") ||
      line.startsWith("index ") ||
      line.startsWith("--- ") ||
      line.startsWith("+++ ") ||
      line.startsWith("similarity index ") ||
      line.startsWith("rename from ") ||
      line.startsWith("rename to ")
    ) {
      // 重命名处理
      if (line.startsWith("rename from ")) {
        currentFile.status = "renamed";
        currentFile.oldPath = line.substring("rename from ".length).trim();
      }
      if (line.startsWith("rename to ")) {
        const newName = line.substring("rename to ".length).trim();
        currentFile.path = newName;
        currentFile.language = detectLanguage(newName);
      }
      continue;
    }

    // 新增/删除文件标记
    if (line.startsWith("new file mode ")) {
      currentFile.status = "added";
      continue;
    }
    if (line.startsWith("deleted file mode ")) {
      currentFile.status = "deleted";
      continue;
    }

    // 二进制文件
    if (line.startsWith("Binary files ")) {
      currentFile.status = "modified";
      continue;
    }

    // Hunk 头: @@ -a,b +c,d @@ context
    if (line.startsWith("@@")) {
      if (currentHunk && currentHunk.lines.length > 0) {
        currentFile.hunks.push(currentHunk);
      }
      currentHunk = { header: line, lines: [] };
      continue;
    }

    if (!currentHunk) continue;

    // Hunk 内容行
    if (line.startsWith("+")) {
      currentHunk.lines.push({
        type: "add",
        content: line.substring(1),
        comments: [],
      });
      additions++;
    } else if (line.startsWith("-")) {
      currentHunk.lines.push({
        type: "remove",
        content: line.substring(1),
        comments: [],
      });
      deletions++;
    } else if (line.startsWith(" ")) {
      currentHunk.lines.push({
        type: "context",
        content: line.substring(1),
        comments: [],
      });
    }
    // No-newline 标记和其他行忽略
  }

  flushFile();
  return files;
}

/** 根据文件路径匹配 DiffFile 列表中的对应项 */
export function findDiffFile(files: DiffFile[], path: string): DiffFile | undefined {
  return files.find(
    (f) => f.path === path || f.oldPath === path
  );
}
