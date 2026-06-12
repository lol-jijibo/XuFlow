/**
 * Xuflow — Ink TUI App
 *
 * 组件树:
 *   App (mode state)
 *   ├── Header        (模型名 · Plan/Act 模式 · token 计数)
 *   ├── MessageList    (对话历史 + 流式输出)
 *   ├── InputBar       (模式名 + 模型名 + 输入框，同一深色底栏)
 *   └── ApprovalModal  (工具审批弹窗)
 */

import React, { useState, useCallback, useMemo, useEffect } from "react";
import { Box, Text, useInput, useStdout } from "ink";
import Spinner from "ink-spinner";
import { useAgent } from "./useAgent.js";
import { getInputCursorAnsi, scheduleInputCursorSync } from "./inputCursor.js";
import type { UIMessage, ToolCallEntry, AgentStatus } from "./useAgent.js";
import type { LLMBackend, AgentMode } from "../types.js";
import type { ConversationMemory } from "../memory/index.js";
import { createBackend } from "../backends/index.js";
import { getModelOptions, type ChatModelOption } from "../modelConfig.js";
import { saveModelPrefs } from "../modelPrefs.js";

// ============================================================
// Props
// ============================================================
interface AppProps {
  backend: LLMBackend;
  systemPrompt: string;
  workspaceName: string;
  memory: ConversationMemory;
}

// ============================================================
// App
// ============================================================
export default function App({
  backend,
  systemPrompt,
  workspaceName: _workspaceName,
  memory,
}: AppProps) {
  const { stdout } = useStdout();
  const terminalHeight = stdout?.rows ?? 40;

  // ---- 模式状态 ----
  const [mode, setMode] = useState<AgentMode>("act");
  const [showModeSelector, setShowModeSelector] = useState(false);
  const [activeBackend, setActiveBackend] = useState<LLMBackend>(backend);
  const [showModelSelector, setShowModelSelector] = useState(false);
  const modelOptions = useMemo(() => getModelOptions(), []);

  const {
    messages,
    status,
    streaming,
    pendingApproval,
    totalTokens,
    sendMessage,
    resolveApproval,
    addMessage,
    resetSession,
  } = useAgent(activeBackend, systemPrompt, mode, memory);

  // ---- Ctrl+T 切换模式 ----
  useInput((input, key) => {
    if (key.ctrl && (input === "\x14" || input === "t" || input === "T")) {
      if (status === "idle") {
        setMode((prev) => (prev === "plan" ? "act" : "plan"));
      }
    }
  });

  // 只显示最近的消息——预估每条消息 ~6 行（含分隔线、标题、正文、工具卡片），
  // 并为 Header(~4) + workspace状态(1) + InputBar(~3) 预留空间
  const visibleMessages = useMemo(() => {
    const overhead = 8;
    const perMsg = 6;
    const maxMsgs = Math.max(Math.floor((terminalHeight - overhead) / perMsg), 2);
    return messages.slice(-maxMsgs);
  }, [messages, terminalHeight]);

  const hasMore = messages.length > visibleMessages.length;

  // ---- 处理用户输入（含斜杠命令） ----
  const handleSubmit = useCallback(
    async (input: string) => {
      const trimmed = input.trim();
      if (!trimmed) return;

      // 斜杠命令
      if (trimmed === "/mode") {
        setShowModeSelector(true);
        return;
      }

      if (trimmed === "/new") {
        try {
          await memory.startNewSession();
        } catch {
          // DB 可能不可用，仍然重置本地状态
        }
        resetSession();
        addMessage("🆕 已开启全新对话");
        return;
      }

      if (trimmed === "/model") {
        setShowModelSelector(true);
        return;
      }

      if (trimmed === "/history") {
        const allMsgs = memory.uiMessages;
        if (allMsgs.length === 0) {
          addMessage("📜 暂无历史消息");
        } else {
          const recent = allMsgs.slice(-20);
          const summary = recent
            .map((m) => {
              const role = m.role === "user" ? "你" : m.role === "assistant" ? "Xuflow" : "系统";
              const preview = m.content.slice(0, 120);
              return `[${role}] ${preview}${m.content.length > 120 ? "..." : ""}`;
            })
            .join("\n");
          addMessage(`📜 当前会话共 ${allMsgs.length} 条消息，最近 ${recent.length} 条:\n${summary}`);
        }
        return;
      }

      if (trimmed.startsWith("/recall ")) {
        const query = trimmed.slice(8).trim();
        if (!query) {
          addMessage("🔍 用法: /recall <关键词>，例如 /recall 错误处理");
        } else {
          const result = await memory.recall(query, 10);
          if (!result) {
            addMessage(`🔍 未找到与 "${query}" 相关的历史消息`);
          } else {
            addMessage(`🔍 搜索 "${query}":\n${result}`);
          }
        }
        return;
      }

      sendMessage(trimmed);
    },
    [sendMessage, addMessage, memory, resetSession]
  );

  return (
    <Box flexDirection="column" height={terminalHeight}>
      <Header
        model={activeBackend.model}
        tokens={totalTokens}
        status={status}
        messageCount={messages.length}
        mode={mode}
      />

      {showModeSelector ? (
        /* 模式选择器 — 居中弹出 */
        <Box flexGrow={1} alignItems="center" justifyContent="center">
          <ModeSelector
            currentMode={mode}
            onSelect={(m) => {
              setMode(m);
              setShowModeSelector(false);
              addMessage(
                `已切换到 ${m === "plan" ? "PLAN" : "ACT"} 模式`
              );
            }}
            onCancel={() => setShowModeSelector(false)}
          />
        </Box>
      ) : showModelSelector ? (
        /* 模型选择器 — 居中弹出 */
        <Box flexGrow={1} alignItems="center" justifyContent="center">
          <ModelSelector
            options={modelOptions}
            currentModel={activeBackend.model}
            onSelect={(option) => {
              setActiveBackend(createBackend(process.env, option));
              setShowModelSelector(false);
              saveModelPrefs({ provider: option.provider, model: option.model });
              addMessage(`已切换到 ${option.label}`);
            }}
            onCancel={() => setShowModelSelector(false)}
          />
        </Box>
      ) : (
        <>
          {/* 消息区 — 占满剩余高度，超出裁剪 */}
          <MessageList
            messages={visibleMessages}
            hasMore={hasMore}
            streaming={streaming}
            status={status}
            mode={mode}
          />
        </>
      )}

      {pendingApproval ? (
        <ApprovalModal
          request={pendingApproval}
          onResolve={resolveApproval}
        />
      ) : (
        <InputBar
          onSubmit={handleSubmit}
          disabled={status !== "idle" || showModeSelector || showModelSelector}
          status={status}
          mode={mode}
          model={activeBackend.model}
        />
      )}
    </Box>
  );
}

// ============================================================
// Header
// ============================================================
function Header({
  model,
  tokens,
  status,
  messageCount,
  mode,
}: {
  model: string;
  tokens: { input: number; output: number };
  status: AgentStatus;
  messageCount: number;
  mode: AgentMode;
}) {
  const statusIcon: Record<AgentStatus, string> = {
    idle: "✓",
    streaming: "●",
    executing: "⚙",
    awaiting_approval: "?",
  };
  const statusColor: Record<AgentStatus, string> = {
    idle: "green",
    streaming: "yellow",
    executing: "blue",
    awaiting_approval: "magenta",
  };

  const modeColor = mode === "plan" ? "yellow" : "cyan";
  const modeLabel = mode === "plan" ? "PLAN" : "ACT";

  return (
    <Box
      flexDirection="column"
      paddingX={1}
      borderStyle="single"
      borderColor="gray"
    >
      {/* 第一行：品牌 + 模型 + 模式 */}
      <Box flexDirection="row" justifyContent="space-between">
        <Box gap={1}>
          <Text bold color="cyan">
            Xuflow
          </Text>
          <Text dimColor>{model}</Text>
        </Box>
        <Box gap={1}>
          <Text color={modeColor} bold>
            {" "}{modeLabel}{" "}
          </Text>
        </Box>
      </Box>
      {/* 第二行：统计 + 状态 */}
      <Box flexDirection="row" justifyContent="space-between">
        <Box gap={2}>
          <Text dimColor>
            {messageCount} 条消息
          </Text>
          <Text dimColor>
            入 {formatTokens(tokens.input)} · 出 {formatTokens(tokens.output)}
          </Text>
        </Box>
        <Text color={statusColor[status]}>
          {statusIcon[status]} {statusLabel(status)}
        </Text>
      </Box>
    </Box>
  );
}

// ============================================================
// Message List
// ============================================================
function MessageList({
  messages,
  hasMore,
  streaming,
  status,
  mode,
}: {
  messages: UIMessage[];
  hasMore: boolean;
  streaming: string;
  status: AgentStatus;
  mode: AgentMode;
}) {
  const emptyHint =
    mode === "plan"
      ? "PLAN — 输入需求，我会分析代码并给出实施计划"
      : "ACT — 输入指令，我会执行代码修改和命令";

  return (
    <Box flexDirection="column" flexGrow={1} paddingX={1} overflow="hidden">
      {hasMore && (
        <Box paddingY={0}>
          <Text dimColor>... 上面还有更多消息 ...</Text>
        </Box>
      )}

      {messages.map((msg, idx) => (
        <Box key={msg.id} flexDirection="column">
          {/* 消息间分隔线（首条消息前不加） */}
          {idx > 0 && (
            <Box>
              <Text dimColor>{"─".repeat(40)}</Text>
            </Box>
          )}
          <MessageBubble msg={msg} />
        </Box>
      ))}

      {streaming && (
        <Box flexDirection="column" marginTop={0}>
          <Text bold color="green">
            🤖 Xuflow
          </Text>
          <MarkdownBody text={streaming} />
          <Text color="yellow">
            <Spinner type="dots" />
          </Text>
        </Box>
      )}

      {messages.length === 0 && !streaming && (
        <Box flexGrow={1} alignItems="center" justifyContent="center">
          <Text dimColor>{emptyHint}</Text>
        </Box>
      )}
    </Box>
  );
}

// ============================================================
// Message Bubble
// ============================================================
function MessageBubble({ msg }: { msg: UIMessage }) {
  const isUser = msg.role === "user";
  const isSystem = msg.role === "system";

  return (
    <Box flexDirection="column" marginY={1}>
      <Text bold color={isUser ? "cyan" : isSystem ? "gray" : "green"}>
        {isUser ? "▸ 你" : isSystem ? "▸ 系统" : "🤖 Xuflow"}
        {msg.usage && !isUser && !isSystem && (
          <Text dimColor>
            {" "}
            ({formatTokens(msg.usage.inputTokens)}→
            {formatTokens(msg.usage.outputTokens)})
          </Text>
        )}
      </Text>

      {msg.content && (
        isSystem ? (
          <Text dimColor>{msg.content}</Text>
        ) : (
          <MarkdownBody text={msg.content} />
        )
      )}

      {msg.toolCalls?.map((tc) => <ToolCallCard key={tc.id} tc={tc} />)}
    </Box>
  );
}

// ============================================================
// Tool Call Card
// ============================================================
function ToolCallCard({ tc }: { tc: ToolCallEntry }) {
  const [expanded, setExpanded] = useState(false);

  const statusIcon: Record<ToolCallEntry["status"], string> = {
    running: "⏳",
    done: "✅",
    denied: "⛔",
    error: "❌",
  };
  const statusColor: Record<ToolCallEntry["status"], string> = {
    running: "yellow",
    done: "green",
    denied: "red",
    error: "red",
  };

  const argsPreview = JSON.stringify(tc.args);
  const argsShort =
    argsPreview.length > 60 ? argsPreview.slice(0, 57) + "..." : argsPreview;

  return (
    <Box flexDirection="column" marginLeft={2}>
      <Text>
        <Text color={statusColor[tc.status]}>
          {statusIcon[tc.status]}{" "}
        </Text>
        <Text color="magenta" bold>
          {tc.name}
        </Text>
        <Text dimColor> {argsShort}</Text>
        {tc.status === "running" && (
          <Text color="yellow">
            {" "}
            <Spinner type="dots" />
          </Text>
        )}
        {tc.result && tc.status !== "running" && (
          <Text color="gray" dimColor>
            {" "}
            [{tc.result.length > 500 && !expanded ? "展开" : "收起"}]
          </Text>
        )}
      </Text>

      {expanded && tc.result && (
        <Box
          flexDirection="column"
          marginLeft={2}
          borderStyle="single"
          borderColor="gray"
          paddingX={1}
        >
          <Text dimColor>{tc.result.slice(0, 2000)}</Text>
          {tc.result.length > 2000 && (
            <Text dimColor>...(结果过长，已截断)</Text>
          )}
        </Box>
      )}
    </Box>
  );
}

// ============================================================
// Markdown Body
//
// 将 Markdown 文本按代码块 / 普通文本分段渲染。
// 普通文本中的每个自然段（以空行分隔）独立占一行，段落内换行保留。
// 代码块使用带边框的独立区块，视觉上与正文区分。
// ============================================================
function MarkdownBody({ text }: { text: string }) {
  const segments = useMemo(() => parseMarkdown(text), [text]);

  return (
    <Box flexDirection="column">
      {segments.map((seg, i) => {
        if (seg.type === "code") {
          return (
            <Box
              key={i}
              flexDirection="column"
              marginY={1}
              borderStyle="single"
              borderColor="gray"
              paddingX={1}
              paddingY={0}
            >
              {seg.content.split("\n").map((line, li) => (
                <Text key={li} dimColor>
                  {line || " "}
                </Text>
              ))}
            </Box>
          );
        }
        // 普通文本：按空行分段，段落间保留间距
        return (
          <Box key={i} flexDirection="column">
            {seg.content
              .split(/\n\n+/)
              .filter((p) => p.trim().length > 0)
              .map((paragraph, pi) => (
                <Box key={pi} marginTop={pi > 0 ? 1 : 0}>
                  {paragraph.split("\n").map((line, li) => (
                    <Text key={li}>{line || " "}</Text>
                  ))}
                </Box>
              ))}
          </Box>
        );
      })}
    </Box>
  );
}

interface MarkdownSegment {
  type: "text" | "code";
  content: string;
}

function parseMarkdown(text: string): MarkdownSegment[] {
  const segments: MarkdownSegment[] = [];
  const lines = text.split("\n");
  let i = 0;

  while (i < lines.length) {
    if (lines[i].trimStart().startsWith("```")) {
      i++;
      const codeLines: string[] = [];
      while (i < lines.length && !lines[i].trimStart().startsWith("```")) {
        codeLines.push(lines[i]);
        i++;
      }
      if (codeLines.length > 0) {
        segments.push({ type: "code", content: codeLines.join("\n") });
      }
      i++;
      continue;
    }

    const textLines: string[] = [];
    while (i < lines.length && !lines[i].trimStart().startsWith("```")) {
      textLines.push(lines[i]);
      i++;
    }
    if (textLines.length > 0) {
      segments.push({ type: "text", content: textLines.join("\n") });
    }
  }

  return segments;
}

// ============================================================
// Custom TextInput — 解决 IME 输入法光标定位问题
//
// ink-text-input 使用反色字符模拟光标（假光标），但真实终端光标
// 停留在渲染输出的末尾。IME 输入法依赖真实终端光标来显示候选窗，
// 所以候选窗总是出现在行尾而不是用户正在编辑的位置。
//
// 本组件在每次渲染后通过 ANSI 转义码 `\x1b[{n}D` 将真实终端光标
// 水平左移到假光标所在位置，使 IME 候选窗显示在正确的位置。
// ============================================================
function TextInput({
  value,
  onChange,
  onSubmit,
  inputStartColumn,
  placeholder = "",
  focus = true,
}: {
  value: string;
  onChange: (value: string) => void;
  onSubmit?: (value: string) => void;
  inputStartColumn: number;
  placeholder?: string;
  focus?: boolean;
}) {
  const { stdout } = useStdout();
  const [cursorOffset, setCursorOffset] = useState(value.length);
  const [cursorWidth, setCursorWidth] = useState(0);

  // 当外部 value 变化时（如提交后清空），重置光标到末尾
  useEffect(() => {
    setCursorOffset(value.length);
    setCursorWidth(0);
  }, [value]);

  // 渲染后将真实终端光标定位到假光标所在列
  // Ink 会渲染完整边框行，需从行首按可视列数重新定位
  useEffect(() => {
    if (!stdout || !focus) return;
    const ansi = getInputCursorAnsi(inputStartColumn, value, cursorOffset);

    return scheduleInputCursorSync(stdout, ansi);
  }, [stdout, focus, inputStartColumn, value, cursorOffset]);

  // ---- 构建渲染值（带假光标） ----
  let renderedValue = value;
  let renderedPlaceholder =
    placeholder.length > 0 ? C.grey(placeholder) : undefined;

  if (focus) {
    renderedPlaceholder =
      placeholder.length > 0
        ? C.inverse(placeholder[0]) + C.grey(placeholder.slice(1))
        : C.inverse(" ");

    if (value.length === 0) {
      renderedValue = C.inverse(" ");
    } else {
      let s = "";
      const end = cursorWidth > 0 ? cursorOffset : cursorOffset;
      for (let i = 0; i < value.length; i++) {
        if (i >= cursorOffset - cursorWidth && i < end) {
          s += C.inverse(value[i]);
        } else {
          s += value[i];
        }
      }
      // 光标在末尾时追加反色空格
      if (cursorOffset === value.length) {
        s += C.inverse(" ");
      }
      renderedValue = s;
    }
  }

  // ---- 键盘输入处理 ----
  useInput(
    (input, key) => {
      // 忽略导航键和特殊组合键
      if (
        key.upArrow ||
        key.downArrow ||
        (key.ctrl && input === "c") ||
        key.tab ||
        (key.shift && key.tab)
      ) {
        return;
      }

      if (key.return) {
        onSubmit?.(value);
        return;
      }

      let nextOffset = cursorOffset;
      let nextValue = value;
      let nextWidth = 0;

      if (key.leftArrow) {
        nextOffset = Math.max(0, cursorOffset - 1);
      } else if (key.rightArrow) {
        nextOffset = Math.min(value.length, cursorOffset + 1);
      } else if (key.backspace || key.delete) {
        if (cursorOffset > 0) {
          nextValue =
            value.slice(0, cursorOffset - 1) +
            value.slice(cursorOffset);
          nextOffset = cursorOffset - 1;
        }
      } else {
        // 普通文本输入（包括 IME 提交后的文字）
        nextValue =
          value.slice(0, cursorOffset) +
          input +
          value.slice(cursorOffset);
        nextOffset = cursorOffset + input.length;
        if (input.length > 1) {
          nextWidth = input.length;
        }
      }

      setCursorOffset(nextOffset);
      setCursorWidth(nextWidth);

      if (nextValue !== value) {
        onChange(nextValue);
      }
    },
    { isActive: focus }
  );

  return (
    <Text>
      {placeholder && value.length === 0
        ? renderedPlaceholder
        : renderedValue}
    </Text>
  );
}

// ---- 内联 ANSI 样式（避免额外依赖 chalk 包） ----
const C = {
  inverse: (s: string) => `\x1b[7m${s}\x1b[27m`,
  grey: (s: string) => `\x1b[90m${s}\x1b[39m`,
};

// ============================================================
// Input Bar — 含模式切换按钮
// ============================================================
function InputBar({
  onSubmit,
  disabled,
  status,
  mode,
  model,
}: {
  onSubmit: (input: string) => void;
  disabled: boolean;
  status: AgentStatus;
  mode: AgentMode;
  model: string;
}) {
  const [value, setValue] = useState("");
  const [suggestionIdx, setSuggestionIdx] = useState(0);

  // ---- 斜杠命令自动提示 ----
  const showSuggestions = value.startsWith("/") && !value.includes(" ");

  const suggestedCommands = useMemo(() => {
    if (!showSuggestions) return [];
    const filter = value.toLowerCase();
    return SLASH_COMMANDS.filter((c) =>
      c.command.toLowerCase().startsWith(filter)
    );
  }, [value, showSuggestions]);

  // 过滤条件变化时重置光标
  useEffect(() => {
    setSuggestionIdx(0);
  }, [value]);

  // 键盘导航（↑↓ Esc）—— Enter 由 handleSubmit 统一处理
  useInput((_input, key) => {
    if (!showSuggestions || suggestedCommands.length === 0) return;

    if (key.upArrow) {
      setSuggestionIdx(
        (prev) =>
          (prev - 1 + suggestedCommands.length) % suggestedCommands.length
      );
    } else if (key.downArrow) {
      setSuggestionIdx((prev) => (prev + 1) % suggestedCommands.length);
    } else if (key.escape) {
      setValue("");
    }
  });

  const handleSubmit = useCallback(
    (val: string) => {
      // 如果当前有 suggestion 且按下 Enter → 直接提交选中的命令
      if (showSuggestions && suggestedCommands.length > 0) {
        const cmd = suggestedCommands[suggestionIdx];
        if (cmd) {
          setValue("");
          onSubmit(cmd.command);
        }
        return;
      }

      const trimmed = val.trim();
      if (!trimmed) return;
      setValue("");
      onSubmit(trimmed);
    },
    [onSubmit, showSuggestions, suggestedCommands, suggestionIdx]
  );

  const placeholder: Record<string, string> = {
    idle: "输入消息...",
    streaming: "Xuflow 正在回复...",
    executing: "正在执行工具...",
    awaiting_approval: "等待审批中...",
  };
  const inputStartColumn = 4;

  return (
    <Box flexDirection="column">
      {/* 斜杠命令自动提示弹窗 */}
      {showSuggestions && suggestedCommands.length > 0 && (
        <Box
          flexDirection="column"
          borderStyle="round"
          borderColor="cyan"
          paddingX={1}
          paddingY={0}
          marginX={1}
        >
          <Text bold color="cyan">
            命令提示
          </Text>
          <Box flexDirection="column" marginTop={0}>
            {suggestedCommands.map((cmd, i) => {
              const isSelected = i === suggestionIdx;
              return (
                <Box key={cmd.command} gap={1}>
                  <Text color={isSelected ? "cyan" : "gray"}>
                    {isSelected ? "❯" : " "}
                  </Text>
                  <Text
                    bold={isSelected}
                    color={isSelected ? "white" : "gray"}
                  >
                    {cmd.command}
                  </Text>
                  <Text dimColor>— {cmd.description}</Text>
                </Box>
              );
            })}
          </Box>
          <Box marginTop={0} gap={2}>
            <Text dimColor>↑↓ 选择</Text>
            <Text dimColor>Enter 确认</Text>
            <Text dimColor>Esc 取消</Text>
          </Box>
        </Box>
      )}

      {/* 输入行 — 全宽深色底栏：模式名 + 模型名在上，提示符 + 输入在下 */}
      <Box
        flexDirection="column"
        backgroundColor="#262626"
        paddingX={1}
        paddingY={1}
      >
        {/* 第一行：模式名 + 模型名 */}
        <Box flexDirection="row">
          <Text bold color={disabled ? "gray" : mode === "plan" ? "yellow" : "cyan"}>
            {mode === "plan" ? "PLAN" : "ACT"}
          </Text>
          <Text dimColor> · </Text>
          <Text bold color="white">
            {model}
          </Text>
          <Box flexGrow={1} />
        </Box>
        {/* 第二行：提示符 + 输入文本 */}
        <Box flexDirection="row" marginTop={0}>
          <Text bold color={disabled ? "gray" : "white"}>
            ▸{" "}
          </Text>
          {disabled ? (
            <Text dimColor>{placeholder[status]}</Text>
          ) : (
            <TextInput
              value={value}
              onChange={setValue}
              onSubmit={handleSubmit}
              inputStartColumn={inputStartColumn}
              placeholder={placeholder.idle}
            />
          )}
          {/* 占位撑满整行 */}
          <Box flexGrow={1} />
        </Box>
      </Box>
    </Box>
  );
}

// ============================================================
// Slash Commands Registry
// ============================================================
interface SlashCommand {
  command: string;
  description: string;
}

const SLASH_COMMANDS: SlashCommand[] = [
  { command: "/mode", description: "切换工作模式（Plan / Act）" },
  { command: "/model", description: "切换当前对话模型" },
  { command: "/new", description: "开始全新对话（重置所有消息）" },
  { command: "/history", description: "查看当前会话历史消息摘要" },
  { command: "/recall", description: "语义搜索历史对话（用法: /recall <关键词>）" },
];

// ============================================================
// Mode Selector — /mode 弹出选择面板
// ============================================================
const MODE_OPTIONS: { value: AgentMode; label: string; key: string }[] = [
  { value: "plan", label: "PLAN  只读 — 只能阅读和分析代码，不可修改文件或执行命令", key: "P" },
  { value: "act", label: "ACT  执行 — 全部工具可用，可写文件和运行命令", key: "A" },
];

function ModeSelector({
  currentMode,
  onSelect,
  onCancel,
}: {
  currentMode: AgentMode;
  onSelect: (mode: AgentMode) => void;
  onCancel: () => void;
}) {
  const [cursor, setCursor] = useState(
    MODE_OPTIONS.findIndex((o) => o.value === currentMode)
  );

  useInput((_input, key) => {
    if (key.upArrow) {
      setCursor((prev) => (prev - 1 + MODE_OPTIONS.length) % MODE_OPTIONS.length);
    } else if (key.downArrow) {
      setCursor((prev) => (prev + 1) % MODE_OPTIONS.length);
    } else if (key.return) {
      onSelect(MODE_OPTIONS[cursor].value);
    } else if (key.escape) {
      onCancel();
    }
  });

  // 字母快捷键
  useInput((input) => {
    const upper = input.toUpperCase();
    const idx = MODE_OPTIONS.findIndex((o) => o.key === upper);
    if (idx !== -1) {
      onSelect(MODE_OPTIONS[idx].value);
    }
  });

  return (
    <Box
      flexDirection="column"
      borderStyle="round"
      borderColor="cyan"
      paddingX={1}
      paddingY={0}
      marginX={1}
    >
      <Text bold color="cyan">
        选择模式
      </Text>
      <Box flexDirection="column" marginTop={0}>
        {MODE_OPTIONS.map((opt, i) => {
          const isSelected = i === cursor;
          const isCurrent = opt.value === currentMode;
          return (
            <Box key={opt.value} gap={1}>
              <Text color={isSelected ? "cyan" : "gray"}>
                {isSelected ? "❯" : " "}
              </Text>
              <Text
                bold={isSelected}
                color={isSelected ? "white" : "gray"}
              >
                {opt.label}
              </Text>
              {isCurrent && (
                <Text dimColor>(当前)</Text>
              )}
            </Box>
          );
        })}
      </Box>
      <Box marginTop={0} gap={2}>
        <Text dimColor>↑↓ 选择</Text>
        <Text dimColor>Enter 确认</Text>
        <Text dimColor>Esc 取消</Text>
        <Text dimColor>P/A 快捷键</Text>
      </Box>
    </Box>
  );
}

// ============================================================
// Model Selector — /model 弹出选择面板
// ============================================================
function ModelSelector({
  options,
  currentModel,
  onSelect,
  onCancel,
}: {
  options: ChatModelOption[];
  currentModel: string;
  onSelect: (option: ChatModelOption) => void;
  onCancel: () => void;
}) {
  const providers = useMemo(() => {
    const uniqueProviders: ChatModelOption["provider"][] = [];
    for (const option of options) {
      if (!uniqueProviders.includes(option.provider)) {
        uniqueProviders.push(option.provider);
      }
    }
    return uniqueProviders;
  }, [options]);
  const [providerCursor, setProviderCursor] = useState(() => {
    const currentOption = options.find((option) => option.model === currentModel);
    const currentIndex = providers.findIndex(
      (provider) => provider === currentOption?.provider
    );
    return currentIndex === -1 ? 0 : currentIndex;
  });
  const [modelCursor, setModelCursor] = useState(() => {
    const currentOption = options.find((option) => option.model === currentModel);
    const currentProvider = currentOption?.provider ?? providers[0];
    const providerOptions = options.filter(
      (option) => option.provider === currentProvider
    );
    const currentIndex = providerOptions.findIndex(
      (option) => option.model === currentModel
    );
    return currentIndex === -1 ? 0 : currentIndex;
  });
  const [activeColumn, setActiveColumn] = useState<"provider" | "model">("model");
  const selectedProvider = providers[providerCursor] ?? providers[0];
  const visibleOptions = options.filter(
    (option) => option.provider === selectedProvider
  );

  useInput((_input, key) => {
    if (key.leftArrow) {
      setActiveColumn("provider");
    } else if (key.rightArrow) {
      setActiveColumn("model");
    } else if (key.upArrow && activeColumn === "provider") {
      setProviderCursor((prev) => (prev - 1 + providers.length) % providers.length);
      setModelCursor(0);
    } else if (key.downArrow && activeColumn === "provider") {
      setProviderCursor((prev) => (prev + 1) % providers.length);
      setModelCursor(0);
    } else if (key.upArrow) {
      setModelCursor((prev) => (prev - 1 + visibleOptions.length) % visibleOptions.length);
    } else if (key.downArrow) {
      setModelCursor((prev) => (prev + 1) % visibleOptions.length);
    } else if (key.return) {
      onSelect(visibleOptions[modelCursor]);
    } else if (key.escape) {
      onCancel();
    }
  }, { isActive: options.length > 0 });

  // 字母快捷键
  useInput((input) => {
    const upper = input.toUpperCase();
    const providerIndex = providers.findIndex(
      (provider) => providerLabel(provider).shortcut === upper
    );
    if (providerIndex !== -1) {
      setProviderCursor(providerIndex);
      setModelCursor(0);
      setActiveColumn("model");
      return;
    }

    const modelIndex = visibleOptions.findIndex((option) => option.key === upper);
    if (modelIndex !== -1) {
      onSelect(visibleOptions[modelIndex]);
    }
  }, { isActive: options.length > 0 });

  return (
    <Box
      flexDirection="column"
      borderStyle="round"
      borderColor="cyan"
      paddingX={1}
      paddingY={0}
      marginX={1}
    >
      <Text bold color="cyan">
        选择模型
      </Text>
      <Box flexDirection="row" gap={4} marginTop={0}>
        <Box flexDirection="column" minWidth={16}>
          {providers.map((provider, i) => {
            const isSelected = i === providerCursor;
            const label = providerLabel(provider);
            return (
              <Box key={provider} gap={1}>
                <Text color={isSelected && activeColumn === "provider" ? "cyan" : "gray"}>
                  {isSelected ? "❯" : " "}
                </Text>
                <Text
                  bold={isSelected}
                  color={isSelected ? "white" : "gray"}
                >
                  {label.name}
                </Text>
              </Box>
            );
          })}
        </Box>
        <Box flexDirection="column">
          {visibleOptions.map((option, i) => {
            const isSelected = i === modelCursor;
            const isCurrent = option.model === currentModel;
            return (
              <Box key={`${option.provider}:${option.model}`} gap={1}>
                <Text color={isSelected && activeColumn === "model" ? "cyan" : "gray"}>
                  {isSelected ? "❯" : " "}
                </Text>
                <Text
                  bold={isSelected}
                  color={isSelected ? "white" : "gray"}
                >
                  {option.label}
                </Text>
                {isCurrent && (
                  <Text dimColor>(当前)</Text>
                )}
              </Box>
            );
          })}
        </Box>
      </Box>
      <Box marginTop={0} gap={2}>
        <Text dimColor>←→ 切换列</Text>
        <Text dimColor>↑↓ 选择</Text>
        <Text dimColor>Enter 确认</Text>
        <Text dimColor>Esc 取消</Text>
        <Text dimColor>D/V 供应商</Text>
      </Box>
    </Box>
  );
}

function providerLabel(provider: ChatModelOption["provider"]): { name: string; shortcut: string } {
  switch (provider) {
    case "volcengine":
      return { name: "火山引擎", shortcut: "V" };
    case "deepseek":
    default:
      return { name: "DeepSeek", shortcut: "D" };
  }
}

// ============================================================
// Approval Modal
// ============================================================
function ApprovalModal({
  request,
  onResolve,
}: {
  request: { tool: string; description: string; command?: string };
  onResolve: (approved: boolean) => void;
}) {
  useEffect(() => {
    const timeout = setTimeout(() => onResolve(false), 180_000);
    return () => clearTimeout(timeout);
  }, [onResolve]);

  useInput((input: string) => {
    const key = input.toLowerCase();
    if (key === "y" || key === "yes") {
      onResolve(true);
    } else if (key === "n" || key === "no" || key === "\r" || key === "\n") {
      onResolve(false);
    }
  });

  return (
    <Box
      flexDirection="column"
      borderStyle="double"
      borderColor="yellow"
      paddingX={1}
      paddingY={0}
      marginX={1}
    >
      <Text bold color="yellow">
        ⚠️ 需要确认: {request.tool}
      </Text>
      <Text>{request.description}</Text>
      {request.command && (
        <Box paddingLeft={2}>
          <Text bold>$ </Text>
          <Text color="yellow">{request.command}</Text>
        </Box>
      )}
      <Box marginTop={0}>
        <Text color="green" bold>
          [Y] 允许
        </Text>
        <Text> · </Text>
        <Text color="red" bold>
          [N/Enter] 拒绝
        </Text>
        <Text dimColor> (3分钟后自动拒绝)</Text>
      </Box>
    </Box>
  );
}

// ============================================================
// Helpers
// ============================================================
function formatTokens(n: number): string {
  if (n >= 1000) return `${(n / 1000).toFixed(1)}K`;
  return String(n);
}

function statusLabel(s: AgentStatus): string {
  switch (s) {
    case "idle":
      return "就绪";
    case "streaming":
      return "回复中";
    case "executing":
      return "执行中";
    case "awaiting_approval":
      return "等待审批";
  }
}
