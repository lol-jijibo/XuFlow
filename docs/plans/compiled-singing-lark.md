# 横屏布局改造方案

## 背景

用户要求将前端项目改为横屏展示，需要同时实现：
1. 移动端强制横屏方向（竖屏时提示旋转）
2. 首页布局从纵向单列改为左右分栏横屏布局
3. 顺手修复之前发现的 5 个 TS 编译错误和 index.html 问题

---

## 实施步骤

### Step 1: 修复 index.html 基础配置
**文件**: `frontend/index.html`
- `lang="en"` → `lang="zh-CN"`
- `<title>frontend</title>` → `<title>智能旅行助手</title>`

### Step 2: 修复 5 个 TS 编译错误（解除构建阻塞）

| 文件 | 改动 |
|------|------|
| `HomeView.vue:32` | 删除 `blurTimer` 变量声明，`setTimeout` 改为裸调用 |
| `PlanView.vue:2` | 从 import 中移除 `onMounted` |
| `PlanView.vue:142` | `v-for="(dayRoute, index)"` → `v-for="dayRoute"` |
| `PlanView.vue:157` | `v-for="(attraction, aIndex)"` → `v-for="attraction"` |
| `PlanCompareView.vue` | 删除未使用的 `useTripStore` import 和 `tripStore` 声明 |

### Step 3: 创建 LandscapeGuard 组件（强制横屏）
**新文件**: `frontend/src/components/LandscapeGuard.vue`

- 检测条件：`matchMedia('(orientation: portrait)')` **且** `matchMedia('(pointer: coarse)')`（同时满足竖屏+触摸设备，避免桌面端误触发）
- 竖屏时：通过 `<Teleport to="body">` 渲染全屏遮罩，显示旋转手机的图标 + "请旋转设备以获得最佳体验" 提示文字
- 旋转图标有 CSS 动画（旋转 90° 循环演示）
- 横屏/桌面端：不渲染任何内容

### Step 4: 在 App.vue 中集成 LandscapeGuard
**文件**: `frontend/src/App.vue`

template 中在 `<router-view />` 前插入 `<LandscapeGuard />`

### Step 5: 改造 HomeView 为左右两栏布局
**文件**: `frontend/src/views/HomeView.vue`

#### 布局策略
```
┌──────────────────────────────────────────────┐
│            header (logo + subtitle)           │
├──────────────────────┬───────────────────────┤
│   左栏：表单面板      │   右栏：预览面板       │
│   (55% 宽度)         │   (45% 宽度)          │
│                      │                       │
│   - 目的地搜索       │   🌍 智能旅行规划      │
│   - 出行日期         │                       │
│   - 旅行偏好         │   📋 行程概览(实时)    │
│   - 预算档位         │   - 目的地             │
│   - 出行人数         │   - 日期/天数          │
│   - 特殊需求         │   - 人数/预算/偏好     │
│   - 🚀 一键生成      │                       │
│                      │   ✨ 功能亮点列表       │
└──────────────────────┴───────────────────────┘
│                 footer                       │
└──────────────────────────────────────────────┘
```

#### CSS 关键点
- `.home-content` 使用 `display: flex; max-width: 1400px; gap: 32px`
- 左栏 `flex: 0 0 55%`，右栏 `flex: 0 0 45%; position: sticky; top: 20px`
- 右栏预览卡片：白色半透明背景 + 阴影 + backdrop-filter
- 地球图标有浮动动画（`translateY` 循环）
- 响应式断点 `@media (max-width: 900px)`：切换回单列纵向布局、隐藏预览面板

#### Script 新增
- `budgetLabel` computed：根据 `budgetLevel` 映射中文标签
- `preferenceLabels` computed：根据已选偏好映射中文标签串

---

## 涉及文件清单

| 操作 | 文件 |
|------|------|
| 新建 | `frontend/src/components/LandscapeGuard.vue` |
| 修改 | `frontend/index.html` |
| 修改 | `frontend/src/App.vue` |
| 修改 | `frontend/src/views/HomeView.vue`（最大改动） |
| 修改 | `frontend/src/views/PlanView.vue` |
| 修改 | `frontend/src/views/PlanCompareView.vue` |

---

## 验证方案

1. `cd frontend && npm run build` — 确保零错误通过
2. `npm run dev` — 浏览器打开
   - 桌面宽屏（>900px）：看到左右两栏布局，右栏实时显示行程概览
   - 缩窄浏览器到 <900px：自动切换为单列布局
   - 移动端竖屏：看到旋转提示遮罩
   - 移动端横屏：正常显示界面
3. 测试表单交互：输入目的地、选日期、切换偏好、切换预算，右栏实时更新
