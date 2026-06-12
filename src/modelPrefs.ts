/**
 * 模型偏好持久化 —— 记住用户上次选择的模型，下次启动自动使用。
 *
 * 存储位置：当前工作目录下的 .xuflow_prefs.json
 * 仅保存 provider + model，不包含 API key 等敏感信息。
 */

import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { resolve } from "node:path";
import type { ChatModelOption } from "./modelConfig.js";

interface ModelPrefs {
  provider: ChatModelOption["provider"];
  model: string;
}

const PREFS_FILE = resolve(process.cwd(), ".xuflow_prefs.json");

export function loadModelPrefs(): ModelPrefs | null {
  try {
    if (!existsSync(PREFS_FILE)) return null;
    const raw = readFileSync(PREFS_FILE, "utf-8");
    const data = JSON.parse(raw);
    if (
      data &&
      typeof data.provider === "string" &&
      typeof data.model === "string"
    ) {
      return { provider: data.provider, model: data.model };
    }
    return null;
  } catch {
    return null;
  }
}

export function saveModelPrefs(prefs: ModelPrefs): void {
  try {
    writeFileSync(PREFS_FILE, JSON.stringify(prefs, null, 2) + "\n", "utf-8");
  } catch {
    // 静默失败 —— 不影响核心功能
  }
}
