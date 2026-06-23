# 侧边栏项目列表折叠/展开交互重构

## Context

当前侧边栏的"项目列表"分组标题行始终可见，无论项目是否展开。用户希望改为：折叠状态下隐藏标题行、仅显示项目节点；展开状态下才显示标题行作为分类导航。同时优化暗色模式下的文字层级和项目行与会话列表之间的间距。

## 修改文件

仅需修改 `desktop/src/components/layout/Sidebar.vue`

## 具体变更

### 1. Script：新增 `hasExpandedProject` 计算属性

- 从 `vue` 导入 `computed`
- 新增计算属性，检测是否有任何项目处于展开状态：

```ts
const hasExpandedProject = computed(() => {
  return Object.values(expanded.value).some(Boolean);
});
```

### 2. Template：条件显示 `.project-header`

在 `.project-header` 的 `<div>` 上添加：

```html
v-if="hasExpandedProject || store.projects.length === 0"
```

逻辑：
- 有项目展开 → 显示标题行（含"项目列表"标签、+ 按钮、全部收起按钮）
- 无项目（空状态）→ 显示标题行（确保 + 按钮可用来创建首个项目）
- 有项目但全部折叠 → 隐藏标题行，仅显示紧凑的项目行列表

### 3. Style：暗色模式"项目列表"标题颜色回调

将 `.sidebar.dark .project-header-title` 的 `color` 从 `#ffffff` 改为 `#9ca3af`（gray-400）。

原因："项目列表"是分类导航标签，应比项目名称（`#ffffff` 纯白）略灰，形成视觉层级。亮色模式下标题 `#94a3b8` 也比项目名 `#374151` 更灰，保持一致的设计语言。

### 4. Style：项目行与会话列表之间增加间距

新增样式块：

```css
.conversation-list {
  padding-top: 4px;
}
```

展开项目时，项目行与其下方会话列表之间产生 4px 间距，明确父子层级关系。

## 验证方式

1. **折叠全部** → 确认"项目列表"标题行消失，仅显示项目行（箭头+图标+名称）
2. **点击展开项目** → 标题行重新出现，箭头旋转，会话列表带间距显示
3. **空项目状态** → 删除所有项目后，标题行仍可见（+ 按钮可用）
4. **暗色模式** → "项目列表"文字呈灰色，项目名保持亮白，层级分明
5. **多项目混合状态** → 部分展开、全部展开、全部折叠三种场景均正确切换
