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
  assert.doesNotMatch(statusBarSource, /鏈湴/, "project label should not include the local suffix");
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
    /class="footbar-left"[\s\S]*class="footbar-model-select"/m,
    "left side should render a borderless NSelect matching the top TitleBar model options"
  );
  assert.match(
    source,
    /\.chat-footbar\s*\{[\s\S]*height:\s*48px;/m,
    "context capacity strip should use the compact height of a status row"
  );
});

test("MessageItem renders assistant content as a left-avatar row with a single right-side text flow", () => {
  const source = readFileSync(
    resolve("desktop/src/components/chat/MessageItem.vue"),
    "utf8"
  );

  assert.match(
    source,
    /class="assistant-message-row"/,
    "assistant message should use a dedicated left-avatar row"
  );
  assert.match(
    source,
    /class="assistant-avatar-column"/,
    "assistant avatar should occupy the left-side column"
  );
  assert.match(
    source,
    /class="assistant-content-column"/,
    "assistant status and answer should share the same right-side column"
  );
  assert.match(
    source,
    /class="assistant-flow"/,
    "assistant content should render as a single vertical flow"
  );
  assert.match(
    source,
    /class="assistant-avatar-glyph"/,
    "assistant row should keep the robot glyph in the avatar column"
  );
  assert.doesNotMatch(
    source,
    /class="agent-block"/,
    "assistant message should no longer use the old centered block wrapper"
  );
  assert.doesNotMatch(
    source,
    /class="tool-calls-block"/,
    "assistant message should no longer split status into a detached tool block"
  );
  assert.doesNotMatch(
    source,
    /class="thinking-done"/,
    "assistant message should no longer show a standalone completion badge row"
  );
});

test("MessageItem keeps status text inside the assistant text flow instead of detached cards", () => {
  const source = readFileSync(
    resolve("desktop/src/components/chat/MessageItem.vue"),
    "utf8"
  );

  assert.match(
    source,
    /const reasoningPrompts = \[/,
    "assistant flow should define a rotating prompt list for the thinking phase"
  );
  assert.match(
    source,
    /思考中\.\.\./,
    "assistant flow should include a localized thinking prompt in the rotation"
  );
  assert.match(
    source,
    /整理回答中\.\.\./,
    "assistant flow should include a localized assembly prompt in the rotation"
  );
  assert.match(
    source,
    /组织语言中\.\.\./,
    "assistant flow should include a localized drafting prompt in the rotation"
  );
  assert.match(
    source,
    /生成回复中\.\.\./,
    "assistant flow should include a localized generation prompt in the rotation"
  );
  assert.match(
    source,
    /reasoningPromptIndex\.value = \(reasoningPromptIndex\.value \+ 1\) % reasoningPrompts\.length/,
    "assistant flow should advance through the thinking prompts in a loop"
  );
  assert.doesNotMatch(
    source,
    /Brewing a reply|Drafting it|Putting it together|Polishing the words|Cooking up an answer|Stitching it up/,
    "assistant flow should no longer show English loading prompts"
  );
  assert.match(
    source,
    /调用工具中\.\.\./,
    "assistant flow should expose a visible tool-running phase"
  );
  assert.match(
    source,
    /if \(isPromptRotatingPhase\.value\) return activeReasoningPrompt\.value;/,
    "assistant flow should reuse the rotating localized prompts for the full non-tool streaming phase"
  );
  assert.match(
    source,
    /工具调用已完成/,
    "assistant flow should keep the tool completion state as inline text"
  );
  assert.match(
    source,
    /class="assistant-flow-status"/,
    "assistant flow should use lightweight status rows inside the text stream"
  );
  assert.doesNotMatch(
    source,
    /class="thinking-indicator"/,
    "assistant message should no longer use the detached indicator row"
  );
});

test("ReasoningBlock becomes a lightweight content section without its own avatar gutter", () => {
  const source = readFileSync(
    resolve("desktop/src/components/chat/ReasoningBlock.vue"),
    "utf8"
  );

  assert.match(
    source,
    /class="reasoning-section"/,
    "reasoning should remain a content section inside the assistant flow"
  );
  assert.match(
    source,
    /class="reasoning-status-button"/,
    "reasoning section should keep a lightweight inline expand toggle"
  );
  assert.doesNotMatch(
    source,
    /class="assistant-robot-svg"/,
    "reasoning section should no longer render a nested avatar icon"
  );
  assert.doesNotMatch(
    source,
    /class="assistant-glyph"/,
    "reasoning section should no longer reserve a separate avatar gutter"
  );
  assert.doesNotMatch(
    source,
    /class="reasoning-layout"/,
    "reasoning section should no longer manage a detached avatar layout"
  );
  assert.match(
    source,
    /\.reasoning-section\s*\{[\s\S]*margin:\s*0;/m,
    "reasoning section should sit flush inside the assistant text flow"
  );
  assert.match(
    source,
    /\.reasoning-content\s*\{[\s\S]*padding:\s*0;/m,
    "reasoning details should expand inline without card padding"
  );
});

test("Assistant avatar and reasoning-complete label share the same first-line height", () => {
  const messageItemSource = readFileSync(
    resolve("desktop/src/components/chat/MessageItem.vue"),
    "utf8"
  );
  const reasoningSource = readFileSync(
    resolve("desktop/src/components/chat/ReasoningBlock.vue"),
    "utf8"
  );

  assert.match(
    messageItemSource,
    /\.assistant-avatar-column\s*\{[\s\S]*width:\s*36px;[\s\S]*padding-top:\s*0;/m,
    "assistant avatar column should grow to 36px without adding a top offset above the reasoning row"
  );
  assert.match(
    messageItemSource,
    /\.assistant-avatar-glyph\s*\{[\s\S]*width:\s*36px;[\s\S]*height:\s*36px;/m,
    "assistant avatar glyph should scale up with the larger robot size"
  );
  assert.match(
    reasoningSource,
    /\.reasoning-status-button\s*\{[\s\S]*min-height:\s*36px;/m,
    "reasoning status row should use the same first-line height as the 36px robot avatar"
  );
});
