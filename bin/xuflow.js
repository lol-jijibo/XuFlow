#!/usr/bin/env node
/**
 * Xuflow CLI launcher — spawns the TUI from the project root.
 *
 * After `npm link`, run `xuflow` anywhere.
 */
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(__dirname, "..");

const child = spawn("npx", ["tsx", "src/loop.ts"], {
  cwd: projectRoot,
  stdio: "inherit",
  shell: true,
});

child.on("close", (code) => process.exit(code ?? 0));
child.on("error", (err) => {
  console.error("❌ 无法启动 Xuflow:", err.message);
  process.exit(1);
});
