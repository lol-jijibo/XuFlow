# 设置页面重新布局 — 实现计划

## Context

当前设置页面 (`/settings`) 复用主页布局：左侧 Sidebar (260px) + 右侧主区域 (TitleBar + 内容 + StatusBar)。用户要求将设置页面改为**全屏独占**，去掉侧边栏和顶部/底部栏，并采用**双栏布局**：左侧为设置大纲导航，右侧为对应详情内容。

## 修改文件清单

仅修改 2 个文件：

### 1. [SettingsView.vue](desktop/src/views/SettingsView.vue) — 重写

**去掉** `Sidebar`、`TitleBar`、`StatusBar` 的引用，改为独立全屏布局：

```
┌──────────────────────────────────────────────────┐
│ ← 返回   设置                          (top bar) │
├──────────────┬───────────────────────────────────┤
│ NLayoutSider │ NLayoutContent                    │
│ (240px)      │                                   │
│              │  SettingsPanel                    │
│  NMenu       │  (按 activeSection 显示对应卡片)  │
│  - API 密钥  │                                   │
│  - 接入点管理│                                   │
│  - 关于      │                                   │
│              │                                   │
└──────────────┴───────────────────────────────────┘
```

**模板结构：**
- `.settings-root` 全屏容器 (`100vw × 100vh`)
- `.settings-topbar` (48px) — 返回箭头按钮 + "设置" 标题
- `NLayout(has-sider)` — 填充剩余空间
  - `NLayoutSider(width=240)` → `NMenu` (垂直菜单, 3 项)
  - `NLayoutContent` → `<SettingsPanel :active-section="activeSection" />`

**Script 新增：**
- `activeSection = ref("api-keys")`
- `menuOptions` 数组，含 inline SVG 图标
- `useRouter` 处理返回导航

### 2. [SettingsPanel.vue](desktop/src/components/config/SettingsPanel.vue) — 轻量重构

**新增 prop：** `defineProps<{ activeSection: string }>()`

**模板变更：**
- 删除 `.settings-header` (标题/副标题/关闭按钮 → 已移到 SettingsView top bar)
- 删除 `NCollapse` 包裹 (接入点区域直接展示)
- 每张 `NCard` 加 `v-if` 条件渲染：
  - API 配置卡片 → `v-if="activeSection === 'api-keys'"`
  - 接入点卡片 → `v-if="activeSection === 'endpoints'"`
  - 关于卡片 → `v-if="activeSection === 'about'"`
- 去除 `max-width: 700px; margin: 0 auto`，内容自然填满右栏

**Script 不变** — 所有逻辑 (`endpointInputs`, `watch`, `applyToBackend` 等) 保持原样。

## 左侧导航项

| key | 标签 | 图标 |
|-----|------|------|
| `api-keys` | API 密钥 | 钥匙 SVG |
| `endpoints` | 接入点管理 | 网格方块 SVG |
| `about` | 关于 | 信息圆圈 SVG |

每个菜单项通过 `renderIcon` 渲染 18×18 inline SVG，匹配项目现有的 `stroke="currentColor"` 风格。

## 交互方式

- **点击菜单切换** (非滚动监听) — `NMenu` 的 `v-model:value` 绑定 `activeSection`
- 使用 `v-if` 而非 `v-show`，切换时销毁/重建表单区域
- 顶栏返回按钮 → `router.push('/')`

## 样式复用

沿用项目现有模式：
- `scoped` CSS，`.dark` class 控制深色模式
- 颜色 token：`#1e293b` / `#e2e8f0` (主文字), `#64748b` / `#94a3b8` (次要), `#101014` / `#1c1c22` (背景)
- 边框：`rgba(0,0,0,0.06)` / `rgba(255,255,255,0.06)`
- 强调色：`#6366f1` (indigo)
- 过渡：`0.3s ease` (背景), `0.15s ease` (hover)

## 验证

1. `npm run dev` (或 Tauri dev) 启动应用
2. 点击侧边栏 "设置" → 确认进入全屏设置页，无 Sidebar/TitleBar/StatusBar
3. 点击左侧菜单项 → 确认右侧内容切换正确
4. 深色/浅色模式切换 → 确认样式正常
5. 修改 API Key 并点 "应用配置" → 确认功能正常
6. 修改接入点 → 确认自动保存
7. 点返回箭头 → 回到主页，布局正常
