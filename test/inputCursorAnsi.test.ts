import assert from "node:assert/strict";
import { setTimeout as wait } from "node:timers/promises";
import {
  getInputCursorAnsi,
  getTerminalTextWidth,
  scheduleInputCursorSync,
} from "../src/ui/inputCursor.js";

assert.equal(getTerminalTextWidth("abc"), 3);
assert.equal(getTerminalTextWidth("中文"), 4);

assert.equal(getInputCursorAnsi(8, "", 0), "\r\x1b[8C");
assert.equal(getInputCursorAnsi(8, "abc", 2), "\r\x1b[10C");
assert.equal(getInputCursorAnsi(8, "中a文", 2), "\r\x1b[11C");

const writes: string[] = [];
scheduleInputCursorSync({ write: (value) => writes.push(value) }, "\r\x1b[8C");
assert.deepEqual(writes, ["\r\x1b[8C"]);
await wait(1);
assert.deepEqual(writes, ["\r\x1b[8C", "\r\x1b[8C"]);
