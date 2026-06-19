# 工具调用展示区重构方案

> 日期：2026-06-19  
> 状态：待审批

---

## 一、Context（背景与问题）

当前聊天界面中，Agent 的每次工具调用都以独立的 `ToolCallCard` 平铺展示。当一轮对话触发 10+ 个工具时（读取、搜索、编辑、Git 操作等），存在三个核心问题：

1. **信息过载且缺乏层级**：平铺列表如同"流水账"，每行包含长路径字符串，用户需扫读大量重复信息
2. **无分组与折叠**：所有卡片独立可折叠，但缺乏整体收起/分组折叠能力，页面被撑得非常长，严重干扰下方答案的阅读
3. **缺乏状态反馈摘要**：仅显示绿色 ✓ 或 ⏳，无法一眼看出"成功获取了什么"（文件行数、搜索结果数、Git 变更摘要等）

### 预期效果

- 将所有工具调用压缩为一行的**摘要条**，默认全部收起
- 按类别分组（文件读取、搜索、Git 等），每组带智能摘要
- 逐级展开：摘要条 → 分组 → 单卡 → 完整结果
- 每个工具卡片头部内联显示结果摘要（如 `📖 loop_.rs — 200 行 ✓`）

---

## 二、工具分类体系

| 类别 | 图标 | 标签 | 包含工具 |
|------|------|------|---------|
| `file_read` | 📖 | 文件读取 | `read_file` |
| `file_write` | ✏️ | 文件编辑 | `write_file`, `edit` |
| `search` | 🔍 | 搜索 | `grep`, `glob` |
| `directory` | 📁 | 目录 | `list_dir` |
| `shell` | 💻 | 命令 | `bash` |
| `web` | 🌍 | 网络 | `web_fetch` |
| `git` | 📊 | Git | `git_status`, `git_diff`, `git_log`, `git_add`, `git_commit` |
| `plan` | 📋 | 规划 | `todo_write`, `propose_plan` |

---

## 三、UI 设计稿

### 全部收起态（默认）

```
┌──────────────────────────────────────────────────────────────┐
│ 📖 3  ✏️ 2  🔍 1  💻 1  📊 2                          ▸ 展开 │
└──────────────────────────────────────────────────────────────┘
```

### 部分展开态

```
┌──────────────────────────────────────────────────────────────┐
│ [📖 3] ✏️ 2  🔍 1  💻 1  📊 2                          ▾ 收起 │
├──────────────────────────────────────────────────────────────┤
│ ▾ 📖 文件读取 (3) — 共 327 行                                │
│   ┌──────────────────────────────────────────────────────┐   │
│   │ ▸ 📖 loop_.rs — 200 行 ✓                             │   │
│   └──────────────────────────────────────────────────────┘   │
│   ┌──────────────────────────────────────────────────────┐   │
│   │ ▸ 📖 mod.rs — 85 行 ✓                                │   │
│   └──────────────────────────────────────────────────────┘   │
│   ┌──────────────────────────────────────────────────────┐   │
│   │ ▸ 📖 settings.json — 42 行 ✓                         │   │
│   └──────────────────────────────────────────────────────┘   │
│                                                              │
│ ▸ ✏️ 文件编辑 (2) — 成功编辑 2 个文件                        │
│ ▸ 🔍 搜索 (1) — 找到 15 处匹配                               │
│ ▸ 💻 命令 (1) — 成功                                         │
│ ▸ 📊 Git (2) — 3 个变更，已提交                              │
└──────────────────────────────────────────────────────────────┘
```

### 设计要点

- **摘要条**：水平排列的 category chip，每 chip 显示图标+计数，可点击切换对应分组
- **分组头**：展开的分组显示图标+标签+计数+组摘要；收起的分组显示单行，含组摘要
- **单卡**：保持现有卡片样式，但头部标签缩短并内联结果摘要
- **颜色编码**：chip 激活态使用类别对应微妙色彩（读取=蓝、编辑=橙、搜索=绿、Git=紫等）
- **过渡动画**：分组展开/收起使用 `max-height` + `opacity` 过渡

---

## 四、实现计划

### 4.1 新建 `desktop/src/utils/toolSummary.ts`

纯函数工具模块，无 Vue 依赖。包含：

```typescript
// ── 类型定义 ──

interface ToolGroup {
  category: ToolCategory;
  icon: string;
  label: string;          // "文件读取"
  entries: ToolCallEntry[];
  summary: string;        // "共 327 行"
}

// ── 分类映射 ──

const TOOL_CATEGORY_MAP: Record<string, ToolCategory> = {
  read_file: 'file_read',
  write_file: 'file_write',
  edit: 'file_write',
  list_dir: 'directory',
  grep: 'search',
  glob: 'search',
  bash: 'shell',
  web_fetch: 'web',
  git_status: 'git',
  git_diff: 'git',
  git_log: 'git',
  git_add: 'git',
  git_commit: 'git',
  todo_write: 'plan',
  propose_plan: 'plan',
};

const CATEGORY_META: Record<ToolCategory, { icon: string; label: string }> = { ... };

// ── 导出函数 ──

/** 将工具调用列表按类别分组，保留原始顺序 */
function groupToolCalls(entries: ToolCallEntry[]): ToolGroup[]

/** 从单个工具结果中提取摘要 */
function summarizeResult(entry: ToolCallEntry): string

/** 生成分组级别的摘要（汇总组内所有工具的结果） */
function summarizeGroup(group: ToolGroup): string
```

#### `summarizeResult` 解析逻辑（按工具名匹配结果字符串）：

| 工具 | 匹配模式 | 摘要示例 |
|------|---------|---------|
| `read_file` | 统计换行数 | `200 行` |
| `write_file` | 正则 `/Successfully wrote (\d+) bytes/` | `1,234 字节` |
| `edit` | 正则 `/(\d+) replacement/` | `2 处替换` |
| `list_dir` | 统计非空行数 | `12 项` |
| `grep` | 统计匹配行（`path:line: content`），或识别 `No matches` | `15 处匹配` / `无匹配` |
| `glob` | 统计路径行，或识别 `No files` | `8 个文件` / `无文件` |
| `bash` | 匹配 `/exit code: (\d+)/` 或统计输出行 | `成功` / `退出码 1` / `50 行输出` |
| `web_fetch` | 统计字符数 | `5,000 字符` |
| `git_status` | 统计非空行，或识别 `clean` | `3 个变更` / `干净` |
| `git_diff` | 统计 `+`/`-` 行首 | `+15/-3 行` |
| `git_log` | 统计提交行 | `5 个提交` |
| `git_add` | 正则 `/Staged (\d+) file/` | `暂存 3 个文件` |
| `git_commit` | 识别 `Committed` | `已提交` |
| `todo_write` | 解析 JSON 统计 todos 数量 | `5 项任务` |
| `propose_plan` | 解析 JSON 统计 steps 数量 | `4 个步骤` |
| 失败（通用） | `resultDone && !result` 或以 `Error`/`Failed` 开头 | `失败` |

#### 边界情况处理：
- `result` 为 `undefined`（工具还在执行中）：返回 `"进行中…"`
- `result` 为空字符串：返回 `"完成"`
- 无法匹配任何模式：返回 `"完成"`（兜底）
- 结构化内容（`todo_write`/`propose_plan` 的 JSON 结果）：安全 `try/catch` 解析

---

### 4.2 增强 `ToolCallCard.vue`

**文件**：`desktop/src/components/chat/ToolCallCard.vue`

改动点：

1. **改进 `shortPath`**：对于文件类工具，只显示文件名（路径最后一段），丢弃目录前缀。当前显示后 3 段（`…/agent/loop_.rs`），改为只显示文件名（`loop_.rs`），将完整路径放入 `title` 属性（hover tooltip）。

2. **新增 `resultSummary` computed**：
   ```typescript
   import { summarizeResult } from "../../utils/toolSummary";
   
   const resultSummary = computed(() => {
     if (!props.entry.resultDone) return "";
     return summarizeResult(props.entry);
   });
   ```

3. **修改 header 模板**：在 `tool-label` 中追加摘要，在 `tool-check` 旁显示：
   ```
   📖 loop_.rs — 200 行 ✓
   ```
   具体模板改动：
   ```html
   <span class="tool-label">{{ label }}</span>
   <span v-if="resultSummary" class="tool-summary">— {{ resultSummary }}</span>
   <span v-if="!entry.resultDone && !entry.result" class="tool-spinner">⏳</span>
   <span v-else-if="entry.resultDone" class="tool-check">✓</span>
   ```

4. **新增 CSS**：`.tool-summary` 样式 — 比 label 更小/更淡，用 `·` 或 `—` 分隔符区分

5. **调整 label 生成**：`shortPath()` 改为只取文件名。例如 `📖 读取 loop_.rs` 而非 `📖 读取 …/packages/core/src/agent/loop_.rs`。完整路径通过 `title` 属性显示。

---

### 4.3 新建 `ToolCallGroup.vue`

**文件**：`desktop/src/components/chat/ToolCallGroup.vue`

功能：渲染一个可折叠的工具分组。

```typescript
// Props
const props = defineProps<{
  group: ToolGroup;
  expanded?: boolean;
}>();

// Local state — synced with parent via watcher
const collapsed = ref(!props.expanded);

watch(() => props.expanded, (val) => {
  collapsed.value = !val;
});
```

**模板结构**：
```html
<div class="tool-group" :class="{ collapsed }">
  <!-- 分组头：始终可见，点击切换 -->
  <div class="group-header" @click="collapsed = !collapsed">
    <span class="group-chevron">{{ collapsed ? '▸' : '▾' }}</span>
    <span class="group-label">{{ group.icon }} {{ group.label }} ({{ group.entries.length }})</span>
    <span class="group-summary">— {{ group.summary }}</span>
  </div>
  
  <!-- 展开的卡片列表 -->
  <div v-if="!collapsed" class="group-body">
    <ToolCallCard
      v-for="tc in group.entries"
      :key="tc.id"
      :entry="tc"
    />
  </div>
</div>
```

**样式要点**：
- `.group-header`：与 `ToolCallCard` header 风格一致（12px padding, hover 背景变化）
- `.group-summary`：比 label 更淡（`color: #888`），字号 11px
- `.group-body`：左边加 2px 竖线缩进指示层级（`border-left: 2px solid rgba(128,128,128,0.15)`），`padding-left: 16px`
- 展开/收起使用 Vue `<Transition>` 或简单的 `max-height` 过渡
- 深色模式适配

---

### 4.4 更新 `MessageItem.vue`

**文件**：`desktop/src/components/chat/MessageItem.vue`

#### 改动 1：新增 import 和分组逻辑

```typescript
import { ref, computed } from "vue";
import ToolCallGroup from "./ToolCallGroup.vue";
import { groupToolCalls } from "../../utils/toolSummary";

// 分组计算
const toolGroups = computed(() => {
  if (!hasToolCalls.value) return [];
  return groupToolCalls(props.message.toolCalls ?? []);
});

// 展开状态：Set 存储当前展开的 category
const expandedCategories = ref<Set<string>>(new Set());
const allExpanded = computed(() => 
  toolGroups.value.length > 0 && expandedCategories.value.size === toolGroups.value.length
);

function toggleCategory(cat: string) {
  const next = new Set(expandedCategories.value);
  if (next.has(cat)) next.delete(cat);
  else next.add(cat);
  expandedCategories.value = next;
}

function toggleAll() {
  if (allExpanded.value) {
    expandedCategories.value = new Set();
  } else {
    expandedCategories.value = new Set(toolGroups.value.map(g => g.category));
  }
}
```

#### 改动 2：替换模板中的 tool-calls-block

旧代码（[MessageItem.vue:49-55](desktop/src/components/chat/MessageItem.vue#L49-L55)）：
```html
<div v-if="hasToolCalls" class="tool-calls-block">
  <ToolCallCard
    v-for="(tc, idx) in message.toolCalls"
    :key="tc.id || idx"
    :entry="tc"
  />
</div>
```

新代码：
```html
<div v-if="hasToolCalls" class="tool-calls-block">
  <!-- 摘要条：始终可见 -->
  <div class="tool-summary-bar">
    <button
      v-for="group in toolGroups"
      :key="group.category"
      class="summary-chip"
      :class="{ active: expandedCategories.has(group.category) }"
      @click="toggleCategory(group.category)"
    >
      {{ group.icon }}&nbsp;{{ group.entries.length }}
    </button>
    <button
      v-if="toolGroups.length > 0"
      class="summary-toggle"
      @click="toggleAll"
    >
      {{ allExpanded ? '▾ 收起' : '▸ 展开' }}
    </button>
  </div>

  <!-- 分组列表 -->
  <div class="tool-groups-list">
    <ToolCallGroup
      v-for="group in toolGroups"
      :key="group.category"
      :group="group"
      :default-expanded="expandedCategories.has(group.category)"
    />
  </div>
</div>
```

#### 改动 3：新增 CSS

- `.tool-summary-bar`：水平 flex 布局，`gap: 8px`，`padding: 8px 0`，可换行
- `.summary-chip`：小药丸按钮 — `padding: 4px 10px`，`border-radius: 12px`，`font-size: 12px`，默认灰色背景，激活态使用类别对应微妙色彩
- `.summary-toggle`：右对齐文字按钮，`font-size: 11px`，`color: #888`
- `.tool-groups-list`：`margin-top: 4px`，`display: flex; flex-direction: column; gap: 4px`

---

### 4.5 样式与动画

**颜色方案（category chip 激活态）**：

| 类别 | 浅色背景 | 深色背景 | 文字色 |
|------|---------|---------|--------|
| file_read | `rgba(59,130,246,0.1)` | `rgba(96,165,250,0.15)` | `#3b82f6` |
| file_write | `rgba(249,115,22,0.1)` | `rgba(251,146,60,0.15)` | `#f97316` |
| search | `rgba(34,197,94,0.1)` | `rgba(74,222,128,0.15)` | `#22c55e` |
| directory | `rgba(168,85,247,0.1)` | `rgba(192,132,252,0.15)` | `#a855f7` |
| shell | `rgba(107,114,128,0.1)` | `rgba(156,163,175,0.15)` | `#6b7280` |
| web | `rgba(14,165,233,0.1)` | `rgba(56,189,248,0.15)` | `#0ea5e9` |
| git | `rgba(236,72,153,0.1)` | `rgba(244,114,182,0.15)` | `#ec4899` |
| plan | `rgba(234,179,8,0.1)` | `rgba(250,204,21,0.15)` | `#eab308` |

**动画**：
- Chip 切换：`transition: all 0.15s ease`
- 分组展开/收起：使用 Vue `<Transition name="group-collapse">`，`max-height` 从 0 到 `auto`，`opacity` 淡入
- 保持与现有 Apple 风格一致（`cubic-bezier(0.25, 0.1, 0.25, 1)`）

---

## 五、文件变更清单

| 操作 | 文件 | 说明 |
|------|------|------|
| **新建** | `desktop/src/utils/toolSummary.ts` | 分类、分组、结果摘要提取（~200 行纯函数） |
| **新建** | `desktop/src/components/chat/ToolCallGroup.vue` | 可折叠工具分组组件（~80 行） |
| **修改** | `desktop/src/components/chat/ToolCallCard.vue` | 标签缩短、内联结果摘要、样式微调（+30 行） |
| **修改** | `desktop/src/components/chat/MessageItem.vue` | 摘要条 + 分组渲染 + 展开状态管理（+100 行） |

**不涉及变更**：Rust 后端、Tauri Bridge、Store、其他前端组件。

---

## 六、数据流示意

```
ToolCallEntry[] (from ChatMessage.toolCalls)
    │
    ├──→ groupToolCalls()          ──→ ToolGroup[] (按类别分组，保留原始顺序)
    │       │
    │       ├──→ summarizeResult()  ──→ 每个 entry 的摘要字符串
    │       └──→ summarizeGroup()   ──→ 每个 group 的摘要字符串
    │
    ├──→ ToolCallSummaryBar        ──→ 水平 chip 条（图标+计数）
    │       └── toggleCategory() / toggleAll()
    │
    └──→ ToolCallGroup[]           ──→ 可折叠分组
            └── ToolCallCard[]     ──→ 单卡（带内联结果摘要）
```

---

## 七、验证方法

1. **单元测试（手动）**：
   - 构造包含 15 种工具（每种至少一个）的 `ToolCallEntry[]`
   - 验证 `groupToolCalls()` 正确分组，保留原始顺序
   - 验证每种工具的 `summarizeResult()` 返回预期摘要
   - 验证边界情况：空结果、错误结果、进行中、超大结果

2. **UI 验证**：
   - 启动应用，发送复杂请求触发多种工具调用
   - 验证默认态：摘要条显示，所有分组收起
   - 验证展开：点击 chip 展开对应分组，点击"展开"展开全部
   - 验证单卡：卡片头部显示内联结果摘要
   - 验证收起：点击已激活 chip 收起分组，点击"收起"收起全部
   - 验证深色模式：切换主题后样式正确

3. **回归验证**：
   - 确认工具调用 ≤ 2 个时，摘要条 + 分组仍然合理（不显空旷）
   - 确认无工具调用时，不渲染摘要条和分组
   - 确认流式工具调用（逐个到达）时，UI 正确更新
