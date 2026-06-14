#!/usr/bin/env node
/**
 * Xuflow CLI — Ink TUI entry point.
 *
 * Usage:
 *   xuflow [--model <provider>] [--api-key <key>] [--base-url <url>]
 */

import React from "react";
import { render, Box, Text, useInput, useApp } from "ink";
import { useState, useEffect, useRef, useCallback } from "react";
import { createRequire } from "node:module";
import { platform, arch } from "node:os";

const require = createRequire(import.meta.url);

// Auto-detect the native addon path based on platform and architecture
function getNativeAddonPath() {
  const platformMap = {
    win32: "win32",
    darwin: "darwin",
    linux: "linux",
  };
  const archMap = {
    x64: "x64",
    arm64: "arm64",
  };
  const plat = platformMap[platform()] || platform();
  const archName = archMap[arch()] || arch();
  // napi-rs uses msvc on Windows, gnu on Linux, and no suffix on macOS
  let triple;
  if (plat === "win32") {
    triple = `win32-${archName}-msvc`;
  } else if (plat === "darwin") {
    triple = `darwin-${archName}`;
  } else {
    triple = `linux-${archName}-gnu`;
  }
  return `../napi/xuflow-napi.${triple}.node`;
}

const napi = require(getNativeAddonPath());

// ─── CLI argument parsing ───────────────────────────────────

const args = process.argv.slice(2);

function getArg(flags, defaultValue) {
  for (const flag of flags) {
    const idx = args.indexOf(flag);
    if (idx !== -1 && idx + 1 < args.length) {
      return args[idx + 1];
    }
  }
  return defaultValue;
}

function hasFlag(flags) {
  return flags.some((f) => args.includes(f));
}

const modelName = getArg(["--model", "-m"], "deepseek-chat");
const apiKey =
  getArg(["--api-key", "-k"]) || process.env.XUFLOW_API_KEY || "";
const baseUrl = getArg(["--base-url", "-u"]);
const provider = getArg(["--provider", "-p"], "deepseek");
const dbPath = getArg(["--db"]);

// ─── Backend factory ────────────────────────────────────────

function createBackend() {
  switch (provider) {
    case "deepseek": {
      const ds = new napi.DeepSeek(modelName, apiKey, baseUrl ?? null);
      return napi.JsBackend.fromDeepseek(ds);
    }
    case "volcengine": {
      const ve = new napi.VolcEngine(modelName, apiKey, baseUrl ?? null);
      return napi.JsBackend.fromVolcengine(ve);
    }
    case "openai":
    default: {
      const oa = new napi.OpenAICompat(
        modelName,
        apiKey,
        baseUrl || "https://api.openai.com/v1"
      );
      return napi.JsBackend.fromOpenaiCompat(oa);
    }
  }
}

// ─── Chat App Component ─────────────────────────────────────

function ChatApp() {
  const { exit } = useApp();
  const [messages, setMessages] = useState([]);
  const [input, setInput] = useState("");
  const [streaming, setStreaming] = useState("");
  const [running, setRunning] = useState(false);
  const [error, setError] = useState(null);
  const [approvalPrompt, setApprovalPrompt] = useState(null);

  const agentRef = useRef(null);
  const sessionRef = useRef(null);
  const sessionIdRef = useRef("");
  const streamRef = useRef("");

  // Initialize agent and session
  useEffect(() => {
    try {
      const tools = new napi.JsToolRegistry();
      const backend = createBackend();
      const agent = new napi.JsAgentLoop(
        backend,
        tools,
        napi.getSystemPrompt()
      );
      agent.setEventCallback((json) => {
        const event = JSON.parse(json);
        handleStreamEvent(event);
      });
      agentRef.current = agent;

      // Initialize session store
      const store = new napi.JsSessionStore(dbPath ?? null);
      sessionRef.current = store;
      const sid = `session_${Date.now()}`;
      sessionIdRef.current = sid;
      store.createSession(sid, "New Chat");
    } catch (e) {
      setError(`Failed to initialize: ${e.message}`);
    }
  }, []);

  const handleStreamEvent = useCallback((event) => {
    switch (event.type) {
      case "TextDelta":
        streamRef.current += event.delta;
        setStreaming((prev) => prev + event.delta);
        break;
      case "ToolCall":
        setMessages((prev) => {
          const last = prev[prev.length - 1];
          if (last && last.role === "assistant" && last.toolCalls) {
            last.toolCalls.push({
              id: event.id,
              name: event.name,
              args: event.arguments,
            });
            return [...prev];
          }
          return [
            ...prev,
            {
              role: "assistant",
              content: "",
              toolCalls: [{ id: event.id, name: event.name, args: event.arguments }],
              toolResults: [],
            },
          ];
        });
        break;
      case "ToolResult":
        setMessages((prev) => {
          const last = prev[prev.length - 1];
          if (last && last.role === "assistant" && last.toolResults) {
            last.toolResults.push({ id: event.id, content: event.content });
            return [...prev];
          }
          return [
            ...prev,
            {
              role: "assistant",
              content: "",
              toolCalls: [],
              toolResults: [{ id: event.id, content: event.content }],
            },
          ];
        });
        break;
      case "ApprovalRequired":
        setApprovalPrompt({ tool: event.tool, params: event.params });
        break;
      case "Done":
        if (streamRef.current.trim()) {
          setMessages((prev) => [
            ...prev,
            { role: "assistant", content: streamRef.current, toolCalls: [], toolResults: [] },
          ]);
        }
        streamRef.current = "";
        setStreaming("");
        setRunning(false);
        break;
      case "Error":
        setError(event.message);
        setRunning(false);
        break;
    }
  }, []);

  const handleSend = useCallback(async () => {
    const text = input.trim();
    if (!text || running || !agentRef.current) return;

    setInput("");
    setError(null);
    setStreaming("");
    streamRef.current = "";
    setRunning(true);

    setMessages((prev) => [
      ...prev,
      { role: "user", content: text, toolCalls: [], toolResults: [] },
    ]);

    try {
      sessionRef.current?.addMessage(sessionIdRef.current, "user", text);
    } catch {}

    try {
      const usageJson = await agentRef.current.run(text);
      const usage = JSON.parse(usageJson);
      sessionRef.current?.addMessage(
        sessionIdRef.current,
        "assistant",
        `[Tokens: ${usage.total_tokens}]`
      );
    } catch (e) {
      setError(e.message || "Agent error");
      setRunning(false);
    }
  }, [input, running]);

  useInput((inputChar, key) => {
    if (key.return) {
      handleSend();
    } else if (key.backspace || key.delete) {
      setInput((prev) => prev.slice(0, -1));
    } else if (inputChar && !key.ctrl && !key.meta) {
      setInput((prev) => prev + inputChar);
    }

    if (key.ctrl && inputChar === "c") {
      exit();
    }
  });

  return React.createElement(
    Box,
    { flexDirection: "column", padding: 1 },
    // Header
    React.createElement(
      Box,
      { marginBottom: 1 },
      React.createElement(Text, { bold: true, color: "cyan" }, "Xuflow"),
      React.createElement(
        Text,
        { dimColor: true },
        ` — ${provider}/${modelName}`
      )
    ),
    // Messages
    React.createElement(
      Box,
      { flexDirection: "column", marginBottom: 1 },
      ...messages.map((msg, i) =>
        React.createElement(
          Box,
          { key: i, flexDirection: "column", marginBottom: 1 },
          React.createElement(
            Text,
            { bold: true, color: msg.role === "user" ? "green" : "blue" },
            msg.role === "user" ? "> " : "\u25CF "
          ),
          msg.content
            ? React.createElement(Text, null, msg.content)
            : null,
          ...(msg.toolCalls || []).map((tc) =>
            React.createElement(
              Box,
              { key: tc.id, marginLeft: 2 },
              React.createElement(
                Text,
                { dimColor: true },
                `\uD83D\uDD27 ${tc.name}(${tc.args.slice(0, 80)})`
              )
            )
          ),
          ...(msg.toolResults || []).map((tr) =>
            React.createElement(
              Box,
              { key: tr.id, marginLeft: 2 },
              React.createElement(
                Text,
                { dimColor: true },
                `\u2514\u2500 ${tr.content.slice(0, 200)}`
              )
            )
          )
        )
      ),
      streaming
        ? React.createElement(
            Box,
            { marginBottom: 1 },
            React.createElement(Text, null, streaming)
          )
        : null,
      running && !streaming
        ? React.createElement(
            Box,
            { marginBottom: 1 },
            React.createElement(Text, { dimColor: true }, "Thinking...")
          )
        : null
    ),
    // Approval prompt
    approvalPrompt
      ? React.createElement(
          Box,
          {
            flexDirection: "column",
            borderStyle: "single",
            borderColor: "yellow",
            padding: 1,
            marginBottom: 1,
          },
          React.createElement(
            Text,
            { bold: true, color: "yellow" },
            "\u26A0 Tool Approval Required"
          ),
          React.createElement(Text, null, `Tool: ${approvalPrompt.tool}`),
          React.createElement(
            Text,
            { dimColor: true },
            `Params: ${approvalPrompt.params}`
          ),
          React.createElement(
            Text,
            null,
            "Press 'y' to approve, 'n' to deny"
          )
        )
      : null,
    // Error
    error
      ? React.createElement(
          Box,
          { marginBottom: 1 },
          React.createElement(Text, { color: "red" }, `Error: ${error}`)
        )
      : null,
    // Input line
    React.createElement(
      Box,
      null,
      React.createElement(
        Text,
        { bold: true, color: "green" },
        "\u276F "
      ),
      React.createElement(Text, null, input),
      running
        ? React.createElement(
            Text,
            { dimColor: true },
            " [running...]"
          )
        : null
    ),
    // Footer
    React.createElement(
      Box,
      { marginTop: 1 },
      React.createElement(
        Text,
        { dimColor: true },
        "Ctrl+C to exit | Enter to send"
      )
    )
  );
}

// ─── Entry ──────────────────────────────────────────────────

if (!apiKey && !hasFlag(["--help", "-h"])) {
  console.error(
    "Error: API key required. Set XUFLOW_API_KEY env var or pass --api-key"
  );
  console.error(
    "Usage: xuflow --provider <deepseek|volcengine|openai> --api-key <key> [--model <name>] [--base-url <url>]"
  );
  process.exit(1);
}

if (hasFlag(["--help", "-h"])) {
  console.log("Xuflow CLI — AI Agent Tool");
  console.log("");
  console.log("Usage: xuflow [options]");
  console.log("");
  console.log("Options:");
  console.log("  -p, --provider   Backend provider: deepseek, volcengine, openai");
  console.log("  -m, --model      Model name (default: deepseek-chat)");
  console.log("  -k, --api-key    API key (or set XUFLOW_API_KEY env var)");
  console.log("  -u, --base-url   Custom API base URL");
  console.log("  --db             SQLite database path for sessions");
  console.log("  -h, --help       Show this help");
  process.exit(0);
}

render(React.createElement(ChatApp));
