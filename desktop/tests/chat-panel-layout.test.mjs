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

test("ChatPanel keeps the input footer visible for empty conversations", () => {
  const source = readFileSync(
    resolve("desktop/src/components/chat/ChatPanel.vue"),
    "utf8"
  );
  const emptyStateIndex = source.indexOf('class="welcome-container"');
  const chatBodyIndex = source.indexOf('class="chat-body"');
  const footerIndex = source.indexOf('class="chat-footer-outer"');

  assert.notEqual(emptyStateIndex, -1, "empty welcome state should still exist");
  assert.notEqual(chatBodyIndex, -1, "non-empty chat body should still exist");
  assert.notEqual(footerIndex, -1, "shared chat footer should still exist");
  assert.ok(
    chatBodyIndex < footerIndex,
    "input footer should be outside the v-else chat body so empty conversations can show it"
  );
  assert.match(
    source,
    /<div v-if="isEmpty" class="welcome-container"[\s\S]*<div v-else class="chat-body"[\s\S]*<\/div> <!-- \/chat-body -->[\s\S]*<div class="chat-footer-outer">/m,
    "empty and non-empty content should share the same input footer"
  );
});

test("Project label stays left while ready status stays right in StatusBar", () => {
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
    /class="project-label"/,
    "chat footer should no longer render the project label"
  );
  assert.match(
    statusBarSource,
    /class="status-left"[\s\S]*class="project-label"[\s\S]*class="project-name"[\s\S]*Xuflow[\s\S]*class="status-right"[\s\S]*class="status-label"[\s\S]*class="version-label"/m,
    "status bar should render project label on the left and ready state on the right"
  );
  assert.doesNotMatch(statusBarSource, /本地/, "project label should not include the local suffix");
  assert.match(
    statusBarSource,
    /\.status-left\s*\{[\s\S]*flex-shrink:\s*0;/m,
    "project label should keep its original left-side status placement"
  );
  assert.match(
    statusBarSource,
    /\.status-right\s*\{[\s\S]*margin-left:\s*auto;/m,
    "ready state should keep its original right-side status placement"
  );
  assert.match(
    statusBarSource,
    /\.project-name\s*\{[\s\S]*background:\s*linear-gradient/m,
    "project name should use a natural gradient treatment"
  );
});

test("StatusBar is taller and extends its background above the labels", () => {
  const source = readFileSync(
    resolve("desktop/src/components/layout/StatusBar.vue"),
    "utf8"
  );

  assert.match(
    source,
    /\.status-bar\s*\{[\s\S]*height:\s*44px;/m,
    "bottom status strip should include the previous gap inside its own background"
  );
  assert.match(
    source,
    /\.status-bar\s*\{[\s\S]*padding:\s*6px 16px 0;/m,
    "bottom status strip should keep label spacing as internal top padding"
  );
  assert.doesNotMatch(
    source,
    /\.status-bar\s*\{[\s\S]*margin-top:/m,
    "status strip should not expose the parent background as a dark band"
  );
});

test("StatusBar label typography scales with the taller strip", () => {
  const source = readFileSync(
    resolve("desktop/src/components/layout/StatusBar.vue"),
    "utf8"
  );

  assert.match(
    source,
    /<svg width="15" height="15"[\s\S]*class="project-icon-svg"/m,
    "project icon should scale up with the taller status strip"
  );
  assert.match(source, /class="project-name"/, "project name should have its own visual treatment");
  assert.match(
    source,
    /\.project-label\s*\{[\s\S]*font-size:\s*12\.5px;/m,
    "project label text should scale up with the taller status strip"
  );
  assert.match(
    source,
    /\.status-label\s*\{[\s\S]*font-size:\s*12\.5px;/m,
    "ready status text should scale up with the taller status strip"
  );
  assert.match(
    source,
    /\.version-label\s*\{[\s\S]*font-size:\s*12px;/m,
    "version text should remain slightly secondary while scaling with the strip"
  );
});

test("ChatPanel context capacity strip follows compact status-bar grouping", () => {
  const source = readFileSync(
    resolve("desktop/src/components/chat/ChatPanel.vue"),
    "utf8"
  );

  assert.match(
    source,
    /class="context-circle-wrap"[\s\S]*class="context-circle-svg"[\s\S]*class="context-tooltip"/m,
    "context capacity should show a minimal circle indicator with hover tooltip"
  );
  assert.match(
    source,
    /class="footbar-right"[\s\S]*class="context-circle-wrap"[\s\S]*class="auto-execute-group"/m,
    "right controls should group context capacity circle with auto-execute toggle"
  );
  assert.match(
    source,
    /class="footbar-left"[\s\S]*class="model-name-label"/m,
    "left side should show the active model name"
  );
  assert.match(
    source,
    /\.chat-footbar\s*\{[\s\S]*height:\s*48px;/m,
    "context capacity strip should use the compact height of a status row"
  );
});
