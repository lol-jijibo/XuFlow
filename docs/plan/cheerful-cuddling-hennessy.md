# 浏览遍历全网 — Phase 1 & 2 实施计划

## Context

Xuflow 当前只有极简陋的 `web_fetch` 工具（手动字符遍历去除 HTML 标签，不支持编码检测，无法提取正文），且没有任何网页搜索能力。本计划实施 Phase 1（增强 web_fetch）和 Phase 2（新增 web_search via Tavily），为后续的 web_crawl 和 deep_research 打好基础。

**目标：**
- `web_fetch` v2：正确解析 HTML DOM + 自动编码检测 + 正文提取
- `web_search`：通过 Tavily API 搜索网页，返回带正文内容的结果

**用户约束：** 不花钱，用 Tavily 免费额度（1000次/月）做测试。

---

## 架构设计

```
packages/core/src/web/          ← 新增 web 基础模块（纯函数，不依赖 Tool trait）
├── mod.rs                       # 公共类型：WebPage, FetchOptions, ExtractMode
├── fetch.rs                     # HTTP 抓取：reqwest + encoding_rs 编码检测
├── extract.rs                   # 内容提取：scraper DOM 解析 + 去噪 + 正文定位
└── search.rs                    # Tavily API 搜索

packages/core/src/tools/
├── web.rs                       # 改写：调用 web::fetch + web::extract
└── web_search.rs                # 新增：调用 web::search::tavily_search
```

**设计原则：**
- `web/` 模块是纯函数库，不依赖 Tool trait，以后 web_crawl/deep_research 也能复用
- `tools/web*.rs` 只做参数解析 + 结果格式化，调用 `web/` 模块的纯函数
- 沿用现有 `Tool` trait 和 `ToolRegistry` 注册模式，不改动接口

---

## 依赖变更

在 `packages/core/Cargo.toml` 新增三个依赖：

```toml
scraper = "0.20"          # HTML DOM 解析（基于 Servo html5ever）
encoding_rs = "0.8"       # 字符编码检测（Firefox 同款）
url = "2"                  # URL 解析与规范化
```

---

## 文件变更清单

### 1. `packages/core/Cargo.toml` — 新增依赖
- 添加 `scraper`、`encoding_rs`、`url` 三个 crate

### 2. `packages/core/src/lib.rs` — 注册新模块
- 添加 `pub mod web;`

### 3. `packages/core/src/web/mod.rs` — 模块入口 + 公共类型（~40 行）
- 定义 `FetchOptions`（timeout, max_chars, user_agent）
- 定义 `WebPage`（url, title, content, status_code）
- 定义 `ExtractMode`（Text / Readability）
- 定义 `SearchResult`（title, url, snippet, content, score）
- 声明子模块

### 4. `packages/core/src/web/fetch.rs` — HTTP 抓取 + 编码检测（~70 行）
- 重构现有 `WebFetchTool::execute` 中的 HTTP 逻辑为独立函数
- `pub async fn fetch_page(url: &str, opts: &FetchOptions) -> Result<WebPage>`
- 流程：
  1. `reqwest` GET 请求（跟随重定向，限制 10 跳）
  2. 检查 Content-Type 是否为 `text/html`
  3. 从 Content-Type header 提取 charset；若无则用 `encoding_rs` 检测原始字节
  4. 解码为 String，提取 `<title>`
  5. 返回 `WebPage { url, title, html_content, status_code }`

### 5. `packages/core/src/web/extract.rs` — 内容提取（~120 行）
- `pub fn extract_content(html: &str, mode: ExtractMode) -> String`
- **Text 模式：** 解析 DOM → 移除 script/style/noscript → 提取 body 所有文本节点 → 合并空白
- **Readability 模式（默认）：**
  1. 解析 DOM
  2. 移除非内容元素（nav, header, footer, aside, script, style, form, iframe, .sidebar, .nav, .footer, .ad, .advertisement）
  3. 定位主内容容器：遍历 body 下的 article/main 标签，或选 `<p>` 标签最多的 DOM 节点
  4. 提取选中容器的纯文本
  5. 合并空白行，截断到 max_chars
- 复用现有 `strip_html_tags` 中的空白规范化逻辑（已实现得不错）

### 6. `packages/core/src/web/search.rs` — Tavily API 搜索（~60 行）
- `pub async fn tavily_search(query: &str, api_key: &str, max_results: u32) -> Result<Vec<SearchResult>>`
- 发送 POST `https://api.tavily.com/search`，参数：
  ```json
  { "api_key": "...", "query": "...", "search_depth": "basic",
    "max_results": 8, "include_answer": true }
  ```
- 解析响应 JSON，映射到 `Vec<SearchResult>`
- Tavily 响应中 `results[].content` 字段已包含提取好的正文

### 7. `packages/core/src/tools/web.rs` — 重写（~100 行）
- 保留 `WebFetchTool` 结构体和 `Tool` trait 实现
- `execute` 方法改为调用 `web::fetch::fetch_page()` + `web::extract::extract_content()`
- 新增参数：
  - `max_chars`（可选，默认 30000）—— 之前是写死 50000
  - `extract_mode`（可选，`"readability"` | `"text"`，默认 `"readability"`）

### 8. `packages/core/src/tools/web_search.rs` — 新增（~70 行）
- 结构体 `WebSearchTool`，实现 `Tool` trait
- 参数：
  - `query`（必填）
  - `max_results`（可选，默认 8）
- `is_dangerous() -> false`
- 读取环境变量 `TAVILY_API_KEY`
- 调用 `web::search::tavily_search()`，格式化结果文本返回

### 9. `packages/core/src/tools/mod.rs` — 注册新模块
- 添加 `pub mod web_search;`

### 10. `cli/napi/src/lib.rs` — CLI 端注册
- 在 `JsToolRegistry::new()` 中添加 `registry.register(Box::new(WebSearchTool));`

### 11. `desktop/src-tauri/src/commands/chat.rs` — Desktop 端注册
- 在 `build_agent()` 中添加 `registry.register(Box::new(WebSearchTool));`

---

## 关键设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| HTML 解析 | `scraper` crate | 基于 Servo html5ever，完整 HTML5 解析，支持 CSS 选择器 |
| 编码检测 | `encoding_rs` | Firefox 内置方案，覆盖所有常见编码，检测准确率最高 |
| 正文提取 | 自研轻量算法 | 没有成熟的 Rust readability crate；用 `p` 密度 + 去噪即可覆盖 90% 场景 |
| 搜索引擎 | Tavily API | 免费 1000次/月，JSON 返回 + 内置正文提取，专为 AI Agent 设计 |
| API Key 管理 | 环境变量 `TAVILY_API_KEY` | 与现有 `KIMI_API_KEY` 等模式一致，后续可在桌面端设置页配置 |

---

## 验证方法

1. **编译检查：** `cargo build -p xuflow-core` 无错误
2. **单元测试：** 对 `extract.rs` 写一个简单的 HTML → 纯文本测试
3. **集成测试：**
   - 设置 `TAVILY_API_KEY` 环境变量
   - CLI: `xuflow` 启动后发送 "帮我搜索 Rust web scraping 最佳实践"
   - 验证 LLM 能调用 `web_search` 工具并返回搜索结果
   - 验证搜索结果中的 URL 能被 `web_fetch` 正确抓取和提取内容
4. **编码测试：** 用 GB2312 编码的中文页面验证 encoding_rs 自动检测

---

## 不做的事情（放到后续阶段）

- ❌ DuckDuckGo HTML 解析（Tavily 完全替代）
- ❌ SearXNG 自建（Tavily 更简单）
- ❌ JavaScript 渲染（Phase 1 范围外）
- ❌ robots.txt 解析（web_crawl 阶段需要）
- ❌ 爬虫 BFS / URL Frontier（Phase 3）
- ❌ deep_research 编排（Phase 4）
