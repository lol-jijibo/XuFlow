/**
 * Xuflow — 工具系统
 *
 * 加新工具:
 *   1. BUILTIN_TOOLS 数组加 ToolDef
 *   2. executeTool switch 加 case
 *   3. 如果需审批 → DANGEROUS_TOOLS Set 加名字
 */

import * as fs from "node:fs";
import * as path from "node:path";
import { exec, execFile } from "node:child_process";
import { promisify } from "node:util";
import type { ToolDef, AgentMode } from "./types.js";

const execAsync = promisify(exec);
const execFileAsync = promisify(execFile);

// ============================================================
// 工具注册表
// ============================================================
export const BUILTIN_TOOLS: ToolDef[] = [
  {
    name: "read_file",
    description:
      "读取一个文件的内容。返回文件文本，带行号前缀。使用场景：查看代码、配置文件、markdown 文档等。",
    parameters: {
      type: "object",
      properties: {
        path: {
          type: "string",
          description: "文件路径，绝对路径或相对于当前工作目录",
        },
      },
      required: ["path"],
    },
  },
  {
    name: "write_file",
    description:
      "写入或覆盖一个文件。使用场景：创建新文件、修改代码后保存。会覆盖已有文件。",
    parameters: {
      type: "object",
      properties: {
        path: {
          type: "string",
          description: "文件路径，绝对路径或相对于当前工作目录",
        },
        content: {
          type: "string",
          description: "要写入的完整文件内容",
        },
      },
      required: ["path", "content"],
    },
  },
  {
    name: "list_dir",
    description: "列出目录中的文件和子目录。使用场景：了解项目结构、查找文件。",
    parameters: {
      type: "object",
      properties: {
        path: {
          type: "string",
          description: "目录路径，默认当前工作目录",
        },
      },
      required: [],
    },
  },
  {
    name: "grep",
    description:
      "在文件内容中搜索匹配的文本或正则表达式。返回匹配行，格式为 file:line: content。使用场景：查找函数定义、引用、错误信息、TODO 注释等。比 list_dir + read_file 组合高效得多。",
    parameters: {
      type: "object",
      properties: {
        pattern: {
          type: "string",
          description: "搜索的正则表达式模式，例如 'function\s+\w+' 或 'TODO'",
        },
        path: {
          type: "string",
          description: "搜索目录，默认当前工作目录",
        },
        glob: {
          type: "string",
          description: "文件名过滤，例如 '*.ts' 或 '*.{js,ts}'",
        },
        head_limit: {
          type: "number",
          description: "最多返回多少行，默认 30",
        },
      },
      required: ["pattern"],
    },
  },
  {
    name: "bash",
    description:
      "在用户终端执行一个 shell 命令并返回输出。使用场景：运行测试、构建项目、git 操作、安装依赖等。此工具需要用户确认后才能执行。",
    parameters: {
      type: "object",
      properties: {
        command: {
          type: "string",
          description: "要执行的 shell 命令",
        },
        description: {
          type: "string",
          description: "简短说明这条命令做什么（用于用户确认时展示）",
        },
      },
      required: ["command", "description"],
    },
  },
];

// ---- 需要用户审批的工具 ----
export const DANGEROUS_TOOLS = new Set(["bash", "write_file"]);

// ---- Plan 模式只读工具白名单 ----
const READONLY_TOOLS = new Set(["read_file", "list_dir", "grep"]);

/** 根据模式返回可用工具 */
export function getToolsForMode(mode: AgentMode): ToolDef[] {
  if (mode === "plan") {
    return BUILTIN_TOOLS.filter((t) => READONLY_TOOLS.has(t.name));
  }
  return BUILTIN_TOOLS; // act 模式：全部工具
}

// ---- bash 危险模式检测 ----
const BASH_DENY_PATTERNS = [
  /rm\s+(-rf?\s+)?\/[^*]/,   // rm -rf /
  /:\s*\{\s*:\s*\|\s*:.*\}/, // fork bomb
  /mkfs\./,                   // 格式化磁盘
  /dd\s+if=/,                 // 磁盘写入
  />\s*\/dev\/sd/,            // 覆盖磁盘设备
  /git\s+push\s+.*--force/,   // force push
  /chmod\s+.*777/,            // 危险权限
];

// ============================================================
// 工具执行器
// ============================================================
export async function executeTool(
  name: string,
  args: Record<string, unknown>
): Promise<string> {
  switch (name) {
    // ---- read_file ----
    case "read_file": {
      const filePath = String(args.path);
      const absPath = path.resolve(filePath);

      if (absPath.includes(".env") || absPath.includes(".git" + path.sep + "config")) {
        return `[denied] 禁止读取敏感文件: ${filePath}`;
      }

      try {
        const content = fs.readFileSync(absPath, "utf-8");
        const lines = content.split("\n");
        const numbered = lines
          .map((line, i) => `${String(i + 1).padStart(4, " ")}\t${line}`)
          .join("\n");
        return `[ok] ${absPath} (${lines.length} 行):\n${numbered}`;
      } catch (err: any) {
        return `[error] 读取失败: ${err.message}`;
      }
    }

    // ---- write_file ----
    case "write_file": {
      const filePath = String(args.path);
      const content = String(args.content);
      const absPath = path.resolve(filePath);

      try {
        fs.mkdirSync(path.dirname(absPath), { recursive: true });
        fs.writeFileSync(absPath, content, "utf-8");
        const lineCount = content.split("\n").length;
        return `[ok] 已写入 ${absPath} (${lineCount} 行, ${content.length} 字节)`;
      } catch (err: any) {
        return `[error] 写入失败: ${err.message}`;
      }
    }

    // ---- list_dir ----
    case "list_dir": {
      const dirPath = String(args.path || ".");
      const absPath = path.resolve(dirPath);

      try {
        const entries = fs.readdirSync(absPath, { withFileTypes: true });
        const lines = entries.map((e) => {
          const type = e.isDirectory() ? "📁" : "📄";
          return `${type} ${e.name}`;
        });
        return `[ok] ${absPath} (${entries.length} 项):\n${lines.join("\n")}`;
      } catch (err: any) {
        return `[error] 列出目录失败: ${err.message}`;
      }
    }

    // ---- grep ----
    case "grep": {
      const pattern = String(args.pattern);
      const searchDir = String(args.path || ".");
      const glob = args.glob ? String(args.glob) : undefined;
      const headLimit = args.head_limit ? Number(args.head_limit) : 30;

      return await grepWithRipgrep(pattern, searchDir, glob, headLimit);
    }

    // ---- bash ----
    case "bash": {
      const command = String(args.command);

      // 危险模式检测
      for (const pattern of BASH_DENY_PATTERNS) {
        if (pattern.test(command)) {
          return `[denied] 命令被安全策略阻止: ${command}`;
        }
      }

      try {
        const { stdout, stderr } = await execAsync(command, {
          cwd: process.cwd(),
          timeout: 120_000, // 2 分钟超时
          maxBuffer: 1024 * 1024, // 1MB 输出上限
          shell: process.platform === "win32" ? "powershell.exe" : "/bin/bash",
        });

        const out = stdout.slice(0, 8000);
        const err = stderr.slice(0, 2000);
        let result = "";
        if (out) result += out;
        if (err) result += (result ? "\n[stderr]\n" : "") + err;
        if (!result) result = "(无输出)";

        return `[ok] $ ${command}\n${result}`;
      } catch (err: any) {
        const stdout = err.stdout?.slice(0, 4000) ?? "";
        const stderr = err.stderr?.slice(0, 4000) ?? "";
        return `[exit ${err.code ?? "?"}] $ ${command}\n${stdout}${stderr}`;
      }
    }

    default:
      return `[error] 未知工具: ${name}`;
  }
}

// ============================================================
// grep 实现：优先 ripgrep，fallback 到 Node.js 遍历
// ============================================================
async function grepWithRipgrep(
  pattern: string,
  dir: string,
  globFilter: string | undefined,
  headLimit: number
): Promise<string> {
  // 尝试 ripgrep
  try {
    const args = ["--line-number", "--no-heading", "--color=never", "-C", "0"];
    if (globFilter) args.push("--glob", globFilter);
    args.push("--", pattern, ".");

    const { stdout } = await execFileAsync("rg", args, {
      cwd: path.resolve(dir),
      timeout: 15_000,
      maxBuffer: 1024 * 1024,
    });

    const lines = stdout.trim().split("\n").filter(Boolean);
    const limited = lines.slice(0, headLimit);
    let result = `[ok] 搜索 "${pattern}" (${lines.length} 个匹配, 显示前 ${limited.length}):\n`;
    result += limited.join("\n") || "(无匹配)";
    return result;
  } catch (err: any) {
    if (err.code === "ENOENT") {
      // rg 未安装，用 Node.js 遍历
      return grepWithNode(pattern, dir, globFilter, headLimit);
    }
    if (err.killed) {
      return `[error] grep 超时 (>15s)，请缩小搜索范围`;
    }
    // rg 退出码 1 = 无匹配，这不是错误
    if (err.code === 1 && !err.stderr) {
      return `[ok] 搜索 "${pattern}": (无匹配)`;
    }
    // 其他错误，也 fallback
    return grepWithNode(pattern, dir, globFilter, headLimit);
  }
}

/** Node.js 实现的文件内容搜索（rg 不可用时的 fallback） */
function grepWithNode(
  pattern: string,
  dir: string,
  globFilter: string | undefined,
  headLimit: number
): Promise<string> {
  const absDir = path.resolve(dir);
  const regex = buildRegex(pattern);
  if (!regex) return Promise.resolve(`[error] 无效的正则表达式: ${pattern}`);

  const results: string[] = [];
  const globRegex = globFilter ? globToRegex(globFilter) : null;
  const re = regex; // TS narrowing fix for nested function

  function walk(currentDir: string) {
    if (results.length >= headLimit) return;

    let entries: fs.Dirent[];
    try {
      entries = fs.readdirSync(currentDir, { withFileTypes: true });
    } catch {
      return; // 跳过无法访问的目录
    }

    for (const entry of entries) {
      if (results.length >= headLimit) return;

      const fullPath = path.join(currentDir, entry.name);

      // 跳过隐藏目录和 node_modules
      if (entry.isDirectory()) {
        if (
          entry.name === "node_modules" ||
          entry.name === ".git" ||
          entry.name === "dist" ||
          entry.name.startsWith(".")
        ) {
          continue;
        }
        walk(fullPath);
        continue;
      }

      // 文件名过滤
      if (globRegex && !globRegex.test(entry.name)) continue;

      // 只搜文本文件（按扩展名简单判断）
      if (isBinary(entry.name)) continue;

      try {
        const content = fs.readFileSync(fullPath, "utf-8");
        const lines = content.split("\n");
        for (let i = 0; i < lines.length; i++) {
          if (results.length >= headLimit) break;
          if (re.test(lines[i])) {
            const relPath = path.relative(absDir, fullPath);
            results.push(`${relPath}:${i + 1}: ${lines[i].trim().slice(0, 200)}`);
          }
        }
      } catch {
        // 跳过无法读取的文件
      }
    }
  }

  walk(absDir);
  return Promise.resolve(
    `[ok] 搜索 "${pattern}" (${results.length} 个匹配):\n${results.join("\n") || "(无匹配)"}`
  );
}

function buildRegex(pattern: string): RegExp | null {
  try {
    return new RegExp(pattern, "gi");
  } catch {
    // 尝试作为字面量字符串搜索
    try {
      return new RegExp(
        pattern.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"),
        "gi"
      );
    } catch {
      return null;
    }
  }
}

function globToRegex(glob: string): RegExp {
  const escaped = glob
    .replace(/\./g, "\\.")
    .replace(/\*/g, ".*")
    .replace(/\?/g, ".");
  return new RegExp(`^${escaped}$`, "i");
}

function isBinary(filename: string): boolean {
  const ext = path.extname(filename).toLowerCase();
  const binaryExts = [
    ".exe", ".dll", ".so", ".dylib", ".bin", ".obj", ".o",
    ".png", ".jpg", ".jpeg", ".gif", ".ico", ".bmp", ".webp",
    ".mp3", ".mp4", ".avi", ".mov", ".mkv", ".wav", ".flac",
    ".zip", ".tar", ".gz", ".rar", ".7z", ".bz2",
    ".pdf", ".doc", ".docx", ".xls", ".xlsx", ".ppt", ".pptx",
    ".ttf", ".otf", ".woff", ".woff2", ".eot",
    ".class", ".pyc", ".wasm",
  ];
  return binaryExts.includes(ext);
}
