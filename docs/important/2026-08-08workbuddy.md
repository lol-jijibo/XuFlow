# 2026-08-08

## 竞品分析
- 完成 XuFlow 与主流 Agent 产品（Claude Code / Cursor / Aider / Cline 等）的差距分析，产出 `docs/竞品分析与功能拓展建议-2026-08-08.md`
- 关键发现：
  - P0 问题：desktop/src-tauri/src/lib.rs 数据库路径硬编码 `D:\Projects-star\Xuflow\...`（release blocker）；上下文为丢弃式裁剪（无 LLM 摘要压缩）；edit 仅字符串替换无 apply_patch；测试极薄（约 18 个）
  - P1 问题：vector.rs 是 TODO（无 RAG/repo map）；无 checkpoint/rewind；CLI 工具注册不全（缺 edit/git/todo/glob），与桌面端能力不对称；无 XUFLOW.md 规则注入
  - 已实现但被高估：MCP 仅 stdio（SSE 未实现）；托盘是 stub；无自动更新；propose_plan 仅是雏形
  - 建议路线：Phase1 正确性底座（路径修复/摘要压缩/apply_patch/测试）→ Phase2 差异化（checkpoint/repo map/slash 命令）→ Phase3 生态（subagent/hooks/多模态/定时调度）
