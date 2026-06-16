# Plan: Slash Command Auto-Suggestions in Xuflow

## Context

Xuflow currently has 3 slash commands (`/mode`, `/plan`, `/act`) that are only matched by exact string comparison. There is no discoverability — users must know the commands in advance. The user wants an auto-suggestion popup that appears when typing `/`, showing all matching English commands with keyboard navigation, similar to VS Code's command palette or existing TUI patterns.

## Design

### Component Architecture

All changes go into a single file: `src/ui/app.tsx`.

The suggestion system lives **inside the `InputBar` component**, keeping it encapsulated. The popup renders between the mode bar and the input row — visually appearing as a floating selection box above the text cursor.

```
InputBar
├── Mode bar (unchanged)
├── CommandSuggestions popup (NEW — conditional)  
└── Input row (unchanged)
```

### New: `SLASH_COMMANDS` Registry

```ts
interface SlashCommand {
  command: string;      // e.g. "/mode"
  description: string;  // e.g. "弹出模式选择面板"
}

const SLASH_COMMANDS: SlashCommand[] = [
  { command: "/mode", description: "弹出模式选择面板" },
  { command: "/plan", description: "切换到 Plan 只读规划模式" },
  { command: "/act",  description: "切换到 Act 执行模式" },
];
```

### Trigger Logic

- `showSuggestions = value.startsWith("/") && !value.includes(" ")`
- When true, filter `SLASH_COMMANDS` by prefix match (case-insensitive `startsWith`)
- When the user types a space (e.g., `/mode some args`), suggestions auto-hide
- When the user clears the input or presses Esc, suggestions hide

### Keyboard Handling

The main technical challenge: both `InputBar`'s `useInput` and `ink-text-input`'s internal `useInput` fire for each keypress. In Ink, parent hooks fire first, child hooks fire second.

**Solution**: Use a `useRef<boolean>` flag (`justSelected`) to bridge the two handlers:

1. InputBar's `useInput` catches ↑/↓/Enter/Esc when suggestions are visible
2. On Enter → set `justSelected.current = true`, update `value` to selected command
3. `handleSubmit` (called by TextInput's internal handler) checks the ref and returns early if true, resetting it to false

This avoids double-submit without needing to modify `ink-text-input`.

### CommandSuggestions Popup (Inline Component)

Styled to match the existing `ModeSelector` popup pattern:
- `borderStyle="round" borderColor="cyan"` — round cyan border
- Title: "命令提示"
- Each row: `❯` cursor indicator + command name (bold on selected) + `— description`
- Footer: `↑↓ 选择  Enter 确认  Esc 取消`
- Cursor resets to 0 when filter changes (via `useEffect` on `value`)

### Input Integration

- When a command is selected (Enter or click-like confirmation), `setValue(cmd.command + " ")` — fills the command with a trailing space so suggestions auto-hide
- The command is NOT auto-submitted — user can review and press Enter to execute
- Existing `handleSubmit` logic (slash command detection in App) works unchanged because the submitted value is exactly the command string

### Existing Behavior Preservation

- `Ctrl+T` mode toggle: unchanged
- ModeSelector popup: unchanged
- ApprovalModal: unchanged
- `/mode`, `/plan`, `/act` command execution in `App.handleSubmit`: unchanged

## Files Modified

| File | Change |
|---|---|
| `src/ui/app.tsx` | Add `SLASH_COMMANDS` registry, add suggestion state+logic in `InputBar`, add `CommandSuggestions` inline component, add `useRef` import |

## Verification

1. Run `npx tsx src/loop.ts`
2. Type `/` — verify the suggestion popup appears with all 3 commands
3. Type `/m` — verify only `/mode` is shown
4. Use ↑/↓ arrows — verify cursor moves and highlight changes
5. Press Enter on a selection — verify the command fills the input with trailing space
6. Press Enter again — verify the command executes (e.g., `/plan` switches mode)
7. Type `/` and press Esc — verify the input clears and popup hides
8. Type a non-command message — verify normal send still works
