# 修复计划：四个核心问题按优先级修复

## Context

经过代码审查，确认以下四个问题均为真实存在的架构缺陷，按严重程度排序：
1. **数据库路径硬编码**（P0 release blocker）—— 换机器直接崩溃
2. **edit 工具只有精确字符串替换** —— 工具调用失败主因
3. **上下文管理"丢弃式裁剪"** —— 长对话 Agent 失忆
4. **测试几乎为零** —— core 包没有安全网

目标是逐一修复这些问题，使项目达到可发布质量。

---

## 修复 1：数据库路径硬编码 → 使用 Tauri app data dir（P0）

**根因**：[desktop/src-tauri/src/lib.rs:20](desktop/src-tauri/src/lib.rs#L20) 写死 `D:\Projects-star\Xuflow\XuFlow-sqlite_content\xuflow.db`

**现状**：`SessionStore::open(Option<PathBuf>)` 已支持 `None` 时走 `default_data_dir()`（见 [session.rs:836-858](packages/core/src/memory/session.rs#L836-L858)），提供跨平台默认路径。问题只在调用侧传了硬编码路径。

**修改方案**：
1. 在 [lib.rs](desktop/src-tauri/src/lib.rs) 的 `setup` 闭包中，通过 `app.path().app_data_dir()` 获取 Tauri 管理的平台应用数据目录
2. 用它替换硬编码的 `PathBuf::from(r"D:\...")`，作为 `db_path` 传入 `SessionStore::open(Some(db_path))`
3. Tauri 的 `app_data_dir` 在不同平台自动解析为：
   - Windows: `%APPDATA%/com.xuflow.app/`
   - macOS: `~/Library/Application Support/com.xuflow.app/`
   - Linux: `~/.local/share/com.xuflow.app/`

**影响范围**：仅 [lib.rs:20](desktop/src-tauri/src/lib.rs#L20) 一行

**验证**：`cargo build -p desktop` 编译通过；非 D 盘环境启动不崩溃

---

## 修复 2：edit 工具增加 apply_patch / unified diff 编辑

**根因**：[edit.rs](packages/core/src/tools/edit.rs) 只有 `old_string` 精确匹配替换，LLM 对空格/缩进偏差敏感，失败率高

**方案**：新增 `apply_patch` 工具，支持 unified diff 格式的 patch 应用

**新增文件**：`packages/core/src/tools/patch.rs`

**工具设计**：
- 工具名：`apply_patch`
- 参数：
  - `path`（必填）：要修改的文件路径
  - `patch`（必填）：unified diff 格式的 patch 内容，使用 `@@ -line,count +line,count @@` hunks
- 实现策略：
  - 解析 unified diff hunks（正则匹配 `@@ -(\d+),?(\d*) \+(\d+),?(\d*) @@`）
  - 逐 hunk 定位原始行并替换，行号对不上时尝试模糊匹配（前后 3 行上下文）
  - 所有 hunks 都成功应用才算成功；任一 hunk 失败则整体回滚并报告具体错误
  - 危险操作（`is_dangerous() = true`），需用户审批
- 这个工具与现有 `edit` 工具互补：
  - `edit`：适合小范围单次替换（重命名、修复一行）
  - `apply_patch`：适合多块编辑、行号定位、对空格缩进容错

**注册**：在 [chat.rs:148](desktop/src-tauri/src/commands/chat.rs#L148) 的 `build_agent` 中注册 `PatchFileTool`

**影响范围**：
- 新文件：`packages/core/src/tools/patch.rs`
- 修改：`packages/core/src/tools/mod.rs`（添加 mod + re-export）
- 修改：`desktop/src-tauri/src/commands/chat.rs`（import + register）

**验证**：在 edit.rs 工具失败的典型场景（带缩进的代码块替换、多行修改）下，apply_patch 用 diff 格式能否成功

---

## 修复 3：上下文压缩 — 丢弃式裁剪 → LLM 摘要压缩

**根因**：[loop_.rs:134-228](packages/core/src/agent/loop_.rs#L134-L228) 的 `trim_context()` 直接丢弃旧 turn，Agent 对早期对话永久失忆

**方案**：在丢弃前先对旧 turn 做 LLM 摘要，将压缩后的摘要作为 system 消息注入

**修改文件**：`packages/core/src/agent/loop_.rs`

**实现策略**：

1. 在 `AgentLoop` 结构体中新增字段：
   - `summary: String` —— 累积的对话摘要
   - `enable_summarization: bool` —— 开关（默认 true），允许禁用

2. 修改 `trim_context()` 逻辑：
   ```
   原逻辑：token > 80% → 丢弃旧 turn → 降至 60%
   新逻辑：token > 80% → 对即将丢弃的 turn 调 LLM 生成摘要 →
          摘要注入为 system 消息（或追加到已有 system prompt）→
          再丢弃旧 turn → 降至 60%
   ```

3. 摘要生成复用已有的 `simple_completion` 模式（见 [chat.rs:569-596](desktop/src-tauri/src/commands/chat.rs#L569-L596)）：
   - 向 backend 发一次无工具调用的 chat
   - prompt 模板："Summarize the following conversation turns, preserving key decisions, code changes, and user intent: ..."
   - 超时 10 秒，失败则降级为原丢弃逻辑

4. 摘要注入方式：将摘要内容追加到 system prompt 末尾（在已有 system 消息后），格式为：
   ```
   [Previous conversation summary]
   {summary_text}
   ```

5. 支持增量摘要：新摘要与旧摘要合并时，对"旧摘要 + 新 turn"再做一次摘要，保持摘要长度可控

**影响范围**：
- [loop_.rs](packages/core/src/agent/loop_.rs)：结构体 + trim_context 方法改造
- [chat.rs](desktop/src-tauri/src/commands/chat.rs)：可能新增 `set_summarization_enabled` Tauri command

**验证**：长对话场景下，检查摘要是否正确生成并注入，Agent 能否引用早期对话内容

---

## 修复 4：测试基础设施

**根因**：core 包总共 18 个测试（9+6+3），tools/ 目录零测试，没有集成测试

**方案**：为核心模块补充单元测试，建立测试目录结构

**新增测试**：

### 4a. tools 模块测试（`packages/core/src/tools/`）
- **edit.rs tests**：[文件内嵌 `#[cfg(test)]` 模块]
  - 单次替换成功
  - 未找到 old_string
  - 多处匹配且未指定 replace_all
  - replace_all 全局替换
  - 文件不存在
- **patch.rs tests**（新增工具附带）：
  - 单 hunk 应用成功
  - 多 hunk 应用成功
  - 行号偏移时的模糊匹配
  - hunk 无法匹配时的错误报告
- **bash.rs tests**：命令执行、超时、危险命令检测

### 4b. agent loop 测试（`packages/core/src/agent/loop_.rs`）
- Token 估算准确性
- trim_context 边界条件
- 摘要生成与注入逻辑

### 4c. session 测试（`packages/core/src/memory/session.rs`）
- 数据库 CRUD
- 软删除/恢复/永久删除
- 回收站过期清理

**测试组织**：
- 单元测试：文件内嵌 `#[cfg(test)]` 模块（与现有 backends/mod.rs 测试风格一致）
- 集成测试：新建 `packages/core/tests/` 目录

**影响范围**：
- 多个 `packages/core/src/` 下的文件增加测试模块
- 新建 `packages/core/tests/integration_test.rs`

**验证**：`cargo test -p xuflow-core` 全部通过，覆盖率显著提高

---

## 执行顺序

1. **修复 1**（P0 路径）→ 1 行改动，立即解除 release blocker
2. **修复 2**（apply_patch 工具）→ 新工具，风险低，独立性强
3. **修复 3**（摘要压缩）→ 改动 loop_.rs 核心逻辑，需要仔细处理
4. **修复 4**（测试）→ 修复 1-3 过程中顺手补测试，最后系统性补充

## 验证方式

```bash
# 修复 1-2 后
cargo build -p xuflow-core
cargo build -p desktop

# 修复 3 后
cargo build -p xuflow-core  # 确保编译通过

# 修复 4 后
cargo test -p xuflow-core   # 全部通过
```
