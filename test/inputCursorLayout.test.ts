import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";

const appSource = readFileSync(path.join(process.cwd(), "src", "ui", "app.tsx"), "utf8");
const textInputIndex = appSource.indexOf("<TextInput");
const inputBoxEndIndex = appSource.indexOf("</Box>", textInputIndex);
const inputTail = appSource.slice(textInputIndex, inputBoxEndIndex);
const inputBarIndex = appSource.indexOf("function InputBar");
const inputBarEndIndex = appSource.indexOf("// ============================================================\r\n// Slash Commands Registry", inputBarIndex);
const inputBarSource = appSource.slice(inputBarIndex, inputBarEndIndex);

assert.equal(
  inputTail.includes("flexGrow={1}"),
  false,
  "输入组件后不能追加弹性空白，否则真实终端光标会停在行尾"
);

assert.equal(
  inputBarSource.includes('const inputStartColumn = 4;'),
  true,
  "输入框只保留左边框、内边距和提示符，真实终端光标应从第 4 列开始"
);

assert.match(
  inputBarSource,
  /<Box[\s\S]*flexDirection="row"[\s\S]*backgroundColor="#262626"[\s\S]*borderStyle="single"[\s\S]*borderColor="#262626"[\s\S]*borderBackgroundColor="#262626"[\s\S]*paddingY=\{0\}[\s\S]*>\s*<Text bold color=\{disabled \? "gray" : "white"\}>\s*▸\{" "\}\s*<\/Text>/,
  "聊天框外层边框和输入行应统一使用输入层深灰色，并贴近单行文字高度"
);

assert.match(
  inputBarSource,
  /<\/Box>\s*<Box marginTop=\{0\} marginLeft=\{2\}>\s*<Text bold color=\{disabled \? "gray" : mode === "plan" \? "yellow" : "cyan"\}>\s*\{mode === "plan" \? "PLAN" : "ACT"\}\s*<\/Text>/,
  "模式标签应放在聊天框外的左下角，ACT 不应成为聊天框内容"
);

assert.equal(
  /backgroundColor="#1f1f1f"[\s\S]*\{mode === "plan" \? "PLAN" : "ACT"\}/.test(inputBarSource),
  false,
  "模式标签不能留在聊天框背景容器内部"
);

assert.equal(
  inputBarSource.includes('borderColor={disabled ? "gray" : mode === "plan" ? "yellow" : "cyan"}'),
  false,
  "聊天框边框不再跟随模式变色，应统一使用深灰色"
);

assert.equal(
  appSource.includes('{ command: "/model", description: "切换当前对话模型" }'),
  true,
  "命令提示应包含 /model，用户可以像 /mode 一样发现模型切换入口"
);

assert.equal(
  appSource.includes("providerCursor") && appSource.includes("modelCursor"),
  true,
  "/model 选择器应包含左侧供应商光标和右侧模型光标，形成两栏选择"
);

const modelSelectorIndex = appSource.indexOf("function ModelSelector");
const modelSelectorEndIndex = appSource.indexOf("// ============================================================\r\n// Approval Modal", modelSelectorIndex);
const modelSelectorSource = appSource.slice(modelSelectorIndex, modelSelectorEndIndex);

assert.equal(
  /<Text dimColor>\{option\.model\}<\/Text>/.test(modelSelectorSource),
  false,
  "模型选择器不应展示真实可调用 ID，只显示友好模型名"
);
