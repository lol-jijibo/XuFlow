# 对比模式双栏展示实施计划

## Context

用户选择了首页的"对比模式"并选择了对比模型后，在答案详情页却没有看到对比效果。原因有两个：
1. **后端对比模型列表不包含主模型**：HomeView 发送 `compare_models` 时只包含用户额外选择的对比模型，不包含主模型（`selectedModel`），导致后端对比模式只生成了对比模型的答案，主模型的答案丢失。
2. **QuestionView 未实现对比模式UI**：store 中的 `compareResults` 数据虽已被正确保存，但 QuestionView 完全不读取这些数据，始终以单栏模式渲染。

## 目标

对比模式下答案详情页改为：
- **左右双栏布局**：左侧 = 聊天框选择的主要模型答案，右侧 = 对比模型答案
- **双目录导航**：左侧答案目录在最左侧，右侧答案目录在最右侧
- **站外亲历链接精简**：仅保留抖音、小红书、知乎三个链接，放在页面最底部
- **API 刷新兼容**：页面刷新或直接访问 URL 时也能正确加载多份答案

## 修改文件清单

### 1. `backend/app/models/schemas.py` — 添加多答案支持
- 在 `QuestionDetail` 中添加可选字段 `compare_answers: list[AnswerResponse] = []`

### 2. `backend/app/api/questions.py` — API 返回所有答案
- 修改 `get_question`：将所有答案（非仅第一条）都返回
- 第一份答案填入 `answer` 字段，其余填入 `compare_answers`

### 3. `frontend/src/types/index.ts` — 前端类型同步
- 在 `QuestionDetail` 中添加 `compare_answers?: AnswerResponse[]`

### 4. `frontend/src/api/client.ts` — API 客户端适配
- `fetchQuestionDetail` 返回类型已从类型定义自动适配（无需改动）

### 5. `frontend/src/stores/question.ts` — Store 增强
- 添加 `compareModelList: ref<string[]>([])` 记录参与对比的模型名称列表（按序）
- 修改 `setCompareStarted` 接收 `models: string[]` 参数
- 添加 getter 方便读取：`compareResultsArray`（按索引排序的结果数组）

### 6. `frontend/src/views/HomeView.vue` — 修复对比模型列表
- **关键修复**：在 `startGeneration` 中，对比模式时将主模型（`activeModel`）也加入 `compare_models` 列表
  ```js
  const allCompareModels = [activeModel, ...compareSelectedModels.value].slice(0, 4)
  ```
- `compare_start` 事件处理中调用 `store.setCompareStarted(true, models_list)`

### 7. `frontend/src/components/AnswerToc.vue` — 支持右侧定位
- 添加 `side?: 'left' | 'right'` prop，默认 `'left'`
- 左侧 TOC 保持现有 `left: ...` 定位
- 右侧 TOC 使用 `right: max(1rem, calc((100vw - 72rem) / 2 - 12rem))` 定位

### 8. `frontend/src/components/ExternalCaseLinks.vue` — 平台过滤
- 添加 `platforms?: CasePlatform[]` prop，默认展示全部（向后兼容）
- 使用 `computed` 根据 `platforms` prop 过滤 `cases` 数组
- 对比模式下传入 `['douyin', 'xiaohongshu', 'zhihu']`

### 9. `frontend/src/views/QuestionView.vue` — 核心改动：双栏布局
- **检测对比模式**：`isCompareView = computed(() => store.isCompareMode && compareResultsArray.value.length >= 2 || questionDetail.value?.compare_answers?.length)`
- **SSE 流路径**：从 `store.compareResults` 读取左右两侧数据
- **API 加载路径**：从 `questionDetail.value.answer`（左）和 `questionDetail.value.compare_answers[0]`（右）读取
- **新增计算属性**：`leftAnswer` / `rightAnswer`（统一 SSE 和 API 两路径）
- **模板结构**（对比模式）：
  ```
  [TOC-左] [左面板: ActionSummary + 正文 + 流程图 + 步骤 + 来源] [右面板: 同上] [TOC-右]
  [底部: ExternalCaseLinks (仅抖音/小红书/知乎)]
  [底部: 相关推荐 / 追问]
  ```
- **单模型模式保持现有布局不变**（向后兼容）

### 10. `frontend/src/assets/main.css` — 可能需要新增样式
- 评估是否需要为双栏对比布局添加专用 CSS（大概率用 Tailwind 工具类即可）

## 数据流总结

```
SSE 流路径:
  HomeView → startGeneration([mainModel, ...compareModels]) → SSE events
  → store.compareResults = {0: mainResult, 1: compareResult, ...}
  → store.compareModelList = ['modelA', 'modelB']
  → router.push('/question/:id')
  → QuestionView 读取 store 渲染双栏

API 刷新路径:
  GET /api/questions/:id → {answer: {...}, compare_answers: [{...}]}
  → QuestionView 从 questionDetail 读取渲染双栏
```

## 验证方式

1. 启动后端和前端开发服务器
2. 在首页选择主模型（如 DeepSeek V4 Pro），开启对比模式，选择对比模型（如 GPT-4o）
3. 提交问题，等待生成完成
4. 验证答案页：
   - 左侧显示主模型答案，右侧显示对比模型答案
   - 左侧目录在最左边，右侧目录在最右边
   - 底部仅显示抖音、小红书、知乎三个站外链接
5. 刷新页面，确认双栏布局仍然正确
6. 在非对比模式下提交问题，确认单栏布局不受影响
