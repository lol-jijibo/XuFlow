# Plan: Merge new/import project buttons into project list header

## Context

The sidebar currently has two separate buttons ("新建项目" and "导入项目") in a `.sidebar-actions` section above the project list. The user wants to:
1. Remove these two separate buttons
2. Replace them with a single folder icon + "+" badge button placed on the **same line as "项目列表"** header
3. Clicking this button shows a dropdown with two options: "新建空白项目" and "使用本地文件"
4. Each option triggers the existing corresponding logic

## Files to Modify

**`packages/desktop/src/components/layout/Sidebar.vue`** — the only file that needs changes.

## Implementation Steps

### 1. Script changes
- Import `NDropdown` from `naive-ui` (add to existing import on line 4)
- Add a `dropdownOptions` array with two items:
  ```ts
  const projectActionOptions = [
    { label: "新建空白项目", key: "create" },
    { label: "使用本地文件", key: "import" },
  ];
  ```
- Add a `handleProjectAction(key: string)` function that switches on key to call `startCreateProject()` or `handleImportProject()`

### 2. Template changes
- **Remove** the entire `<div class="sidebar-actions">` block (lines 123-161) — the two NButtons
- **Add** a dropdown trigger button inside `.project-header`, after the existing `<span class="project-header-title">项目列表</span>` and `<div class="project-header-actions">`:
  ```html
  <NDropdown trigger="click" :options="projectActionOptions" @select="handleProjectAction">
    <NButton size="tiny" quaternary class="add-project-btn">
      <template #icon>
        <!-- folder icon with + badge overlay -->
      </template>
    </NButton>
  </NDropdown>
  ```

### 3. Icon design
The button icon is a composite SVG: a folder icon with a small "+" badge in the corner:
- Folder SVG (16x16) at the center
- A small circle with "+" overlaying the bottom-right corner

### 4. Style changes
- Remove `.sidebar-actions` and `.action-btn` CSS rules
- Add `.add-project-btn` styling — make it always visible, positioned in the header row

## Key Design Decisions
- Use Naive UI's `NDropdown` with `trigger="click"` for the popup menu — it's already a project dependency, no new library needed
- The button sits alongside the existing "全部收起" button in the `.project-header` row, so both are visible when hovering — or make the new button always visible
- Remove the `.sidebar-actions` section entirely, including its CSS

## Verification
1. The two old buttons should no longer appear above the project list
2. The new folder+icon button should appear in the project header row, on the same line as "项目列表"
3. Clicking the button should show a dropdown with "新建空白项目" and "使用本地文件"
4. Selecting "新建空白项目" should show the inline name input (existing behavior)
5. Selecting "使用本地文件" should open the directory picker dialog (existing behavior)
