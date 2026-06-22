# Phase 3: web_crawl — BFS 爬虫实施计划

## Context

Phase 1 (增强 web_fetch) 和 Phase 2 (新增 web_search) 已完成。Phase 3 要实现 `web_crawl`：基于 BFS 的网页爬虫，支持 URL Frontier、robots.txt 遵守、同域限制、链接深度控制。底层复用已有的 `web::fetch::fetch_page()` 和 `web::extract::extract_content()` 纯函数。

**设计原则：**
- `web/crawl.rs` 是纯函数库，不依赖 Tool trait，后续 deep_research 可直接复用
- `tools/web_crawl.rs` 只做参数解析 + 结果格式化，调用 `web::crawl` 模块
- 沿用现有 `Tool` trait 和 `ToolRegistry` 注册模式
- 可用 `url = "2"` crate（已在 Cargo.toml 但未使用）

## 依赖变更

在 `packages/core/Cargo.toml` 新增一个依赖：
```toml
robotstxt = "0.10"    # robots.txt 解析器，遵守网站爬取协议
```

## 文件变更清单

### 1. `packages/core/Cargo.toml` — 新增 robotstxt 依赖
- 在 `url = "2"` 行后添加 `robotstxt = "0.10"`

### 2. `packages/core/src/web/mod.rs` — 新增类型 + 模块声明
- 添加 `pub mod crawl;`
- 新增 3 个结构体：`CrawlConfig`、`CrawledPage`、`CrawlResult`

```rust
/// 网页爬虫 BFS 配置。
#[derive(Debug, Clone)]
pub struct CrawlConfig {
    pub seed_url: String,
    pub max_depth: u32,           // 默认 3
    pub max_pages: u32,           // 默认 20
    pub same_domain_only: bool,   // 默认 true
    pub respect_robots_txt: bool, // 默认 true
    pub request_delay_ms: u64,    // 默认 1000
    pub timeout_secs: u64,        // 默认 30
    pub max_chars_per_page: usize,// 默认 5000
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CrawledPage {
    pub url: String,
    pub title: String,
    pub content: String,
    pub depth: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CrawlResult {
    pub seed_url: String,
    pub pages_crawled: u32,
    pub pages_skipped: u32,
    pub pages: Vec<CrawledPage>,
    pub logs: Vec<String>,
}
```

`CrawlConfig` 提供 `Default` 实现（seed_url 需调用方设置）。

### 3. `packages/core/src/web/crawl.rs` — 核心 BFS 爬虫（~250 行）

**公开函数：**
```rust
pub async fn crawl(config: &CrawlConfig) -> Result<CrawlResult>
```

**内部辅助函数：**
- `normalize_url_str(url: &Url) -> String` — 去 fragment、去尾随斜杠（保留根路径 `/`）、小写化
- `extract_links(html: &str, base_url: &Url, seed_domain: &str, same_domain_only: bool) -> Vec<Url>` — 用 `scraper::Selector::parse("a[href]")` 提取链接，过滤非 http/https、解析相对 URL
- `fetch_robots_txt(client: &reqwest::Client, domain: &str, scheme: &str) -> Option<robotstxt::Robotstxt>` — 抓取 robots.txt，失败返回 None（视为允许全部）
- `is_url_allowed(url: &Url, robots: &robotstxt::Robotstxt) -> bool` — 用 `DefaultMatcher` 检查路径

**BFS 算法：**
```
1. 解析 seed_url → seed_domain
2. 初始化 visited(HashSet), queue(VecDeque), robots_cache(HashMap)
3. 构建 reqwest::Client（复用连接池）
4. BFS 循环直到 queue 为空或 pages.len() >= max_pages:
   a. 出队 (url, depth)
   b. 归一化去重检查 → 重复则跳过
   c. 深度检查 → 超过 max_depth 则跳过
   d. 同域检查 → 不同域则记录日志、递增 skip 计数
   e. robots.txt 检查 → 懒加载缓存，被拦截则记录日志
   f. 请求延迟（首请求不延迟）
   g. 调用 web::fetch::fetch_page() 抓取
   h. 记录原始及最终 URL（处理重定向去重）
   i. 调用 web::extract::extract_content(Readability) 提取正文
   j. 存入 pages[]
   k. 若 depth < max_depth，提取链接并入队
5. 返回 CrawlResult
```

**错误处理：** 单页抓取失败不中断整次爬取，仅记录日志继续下一个 URL。仅 seed_url 本身不合法（无法解析、非 http/https）时才返回顶层 Err。

**URL 归一化策略：** 去 fragment + 去尾随斜杠 /path/ → /path（保留 /）+ 全小写。用 `url` crate 的 `Url::parse()` 和 `base.join()` 处理相对链接解析。

**robots.txt 集成：** 按域懒加载缓存到 `HashMap`。抓取失败（404/timeout）视为允许全部。用 `DefaultMatcher::allowed(path, "Xuflow", robots)` 验证。

### 4. `packages/core/src/tools/web_crawl.rs` — Tool 包装器（~100 行）

```rust
pub struct WebCrawlTool;

// impl Tool:
//   name: "web_crawl"
//   description: "爬取网站，从种子 URL 出发按 BFS 抓取多个页面并提取正文"
//   is_dangerous: false
//   parameters: {
//     url (required), max_depth (optional, default 3),
//     max_pages (optional, default 20),
//     same_domain_only (optional, default true),
//     max_chars_per_page (optional, default 5000)
//   }
//   execute: 验证 URL scheme → 构建 CrawlConfig → crawl() → 格式化为 Markdown
```

**结果格式化：** 摘要区（种子 URL / 抓取数 / 跳过数 / 深度 / 域限制）+ 日志列表 + 每页正文（以 `---` 分隔）。

### 5. `packages/core/src/tools/mod.rs` — 注册模块
- 添加 `pub mod web_crawl;`

### 6. `packages/core/src/agent/system_prompt.rs` — 补充工具说明
- 追加 `- web_search: Search the internet and return results with content.`（缺失项）
- 追加 `- web_crawl: Crawl a website starting from a seed URL, following links to extract content from multiple pages.`

### 7. `desktop/src-tauri/src/commands/chat.rs` — Desktop 端注册
- 第 14 行 import：添加 `web_crawl::WebCrawlTool`
- `build_agent()` 中：`registry.register(Box::new(WebCrawlTool));`

### 8. `cli/napi/src/lib.rs` — CLI 端注册
- import 块：添加 `web_crawl::WebCrawlTool`
- `JsToolRegistry::new()` 中：`registry.register(Box::new(WebCrawlTool));`

## 验证方法

1. **编译检查：** `cargo build -p xuflow-core` 无错误
2. **单元测试：** 在 `crawl.rs` 中添加 URL 归一化测试、链接提取测试
3. **集成测试：**
   - 启动桌面端，发送 "爬取 https://docs.rs/scraper/latest/scraper/ 深度1最多5页"
   - 验证 Agent 能调用 `web_crawl` 工具并返回多页正文
   - 验证同域过滤生效（不应出现 crates.io 等外链的页面）
4. **robots.txt 测试：** 爬取 `https://github.com/rust-lang`，验证 robots.txt 缓存和路径检查逻辑

## 不做的事情（Phase 4+ 范围）

- ❌ 深度研究编排（deep_research）
- ❌ 增量去重（内容相似度哈希）
- ❌ 动态调度 / URL 优先级排序
- ❌ JavaScript 渲染（headless browser）
- ❌ 分布式爬取
- ❌ 站点地图（sitemap.xml）解析
