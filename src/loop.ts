/**
 * Xuflow 鈥?Entry Point
 *
 * 启动 Ink TUI，接管整个终端。
 * 运行: npx tsx src/loop.ts
 * 环境: 仅需要在环境变量中配置 DEEPSEEK_API_KEY 等 LLM 头部信息。
 */

import "dotenv/config";
import { render } from "ink";
import React from "react";
import App from "./ui/app.js";
import { createBackend } from "./backends/index.js";
import { getWorkspaceDisplayName } from "./ui/workspaceStatus.js";
import { createConversationMemory } from "./memory/index.js";
import { loadModelPrefs } from "./modelPrefs.js";

const workspacePath = process.cwd();
const workspaceName = getWorkspaceDisplayName(workspacePath);
const memory = createConversationMemory({
  workspacePath,
});
await memory.ready;

// 优先使用上次保存的模型偏好，没有则回退到环境变量默认值
const savedPrefs = loadModelPrefs();
const backend = createBackend(
  process.env,
  savedPrefs
    ? { provider: savedPrefs.provider, model: savedPrefs.model }
    : undefined
);

const SYSTEM_PROMPT = `你是 Xuflow，一个 AI 编程助手，运行在用户终端中。
可用工具:
- read_file: 读取文件内容，返回带行号的文本
- write_file: 写入/覆盖文件，需要用户确认
- list_dir: 列出目录内容
- grep: 搜索代码内容，返回 file:line: 匹配行
- bash: 执行 shell 命令，需要用户确认
工作原则:
1. 先搜索再编辑，用 grep 定位相关代码，用 read_file 读取上下文，最后才 write_file
2. bash 命令要写清楚 description 参数，让用户知道你在做什么
3. 回答使用中文，代码块使用 markdown 语法
4. 保持简洁，终端里不需要长篇大论`;

const { waitUntilExit } = render(
  React.createElement(App, {
    backend,
    systemPrompt: SYSTEM_PROMPT,
    workspaceName,
    memory,
  })
);

waitUntilExit().then(() => {
  // 正常退出时不做额外处理。
});
