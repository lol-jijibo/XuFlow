import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

test("ChatPanel keeps footer overlay full width while centering input on max-w-4xl", () => {
  const filePath = resolve("desktop/src/components/chat/ChatPanel.vue");
  const source = readFileSync(filePath, "utf8");

  assert.match(
    source,
    /class="chat-content-shell max-w-4xl mx-auto"/,
    "message and footer shells should share the max-w-4xl mx-auto container"
  );
  assert.match(
    source,
    /class="chat-footer-shell chat-content-shell max-w-4xl mx-auto"/,
    "footer should use a dedicated centered shell inside the full-width overlay"
  );
  assert.match(
    source,
    /\.chat-footer-outer\s*\{[\s\S]*width:\s*100%;/m,
    "footer overlay should remain full width"
  );
});

test("Project label moves to StatusBar and ready status sits before the version", () => {
  const chatPanelSource = readFileSync(
    resolve("desktop/src/components/chat/ChatPanel.vue"),
    "utf8"
  );
  const statusBarSource = readFileSync(
    resolve("desktop/src/components/layout/StatusBar.vue"),
    "utf8"
  );

  assert.doesNotMatch(
    chatPanelSource,
    /Xuflow 本地/,
    "chat footer should no longer render the project label"
  );
  assert.match(
    statusBarSource,
    /class="project-label"/,
    "status bar should render the project label on the left"
  );
  assert.match(
    statusBarSource,
    /class="status-right"[\s\S]*class="status-label"[\s\S]*class="version-label"/m,
    "status text should render before the version label on the right"
  );
});
