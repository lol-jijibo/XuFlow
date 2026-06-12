export function getTerminalTextWidth(text: string): number {
  let width = 0;

  for (const char of text) {
    const codePoint = char.codePointAt(0) ?? 0;
    width += codePoint > 0xff ? 2 : 1;
  }

  return width;
}

export function getInputCursorAnsi(
  inputStartColumn: number,
  value: string,
  cursorOffset: number
): string {
  const visibleOffset = getTerminalTextWidth(value.slice(0, cursorOffset));
  const cursorColumn = inputStartColumn + visibleOffset;

  return cursorColumn > 0 ? `\r\x1b[${cursorColumn}C` : "\r";
}

export function scheduleInputCursorSync(
  output: { write: (value: string) => void },
  ansi: string
): () => void {
  let cancelled = false;

  // 延迟写入，让 Ink 先完成本次渲染再移动光标。
  // 避免在终端 resize 等重绘过程中直接写 stdout 导致乱码。
  const timer = setTimeout(() => {
    if (!cancelled) {
      output.write(ansi);
    }
  }, 0);

  return () => {
    cancelled = true;
    clearTimeout(timer);
  };
}
