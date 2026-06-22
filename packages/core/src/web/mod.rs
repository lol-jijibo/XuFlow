// 网页抓取与搜索引擎的公共类型定义。
// 本模块是纯函数库，不依赖 Tool trait，供 tools/ 和后续 crawl/research 复用。

pub mod crawl;
pub mod fetch;
pub mod extract;
pub mod search;

/// 内容提取模式：纯文本去标签 vs 智能定位正文区域。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractMode {
    /// 仅去除 HTML 标签，保留所有可见文本（含导航、页脚等噪音）。
    Text,
    /// 智能定位页面主内容区域，剔除导航/侧边栏/广告等噪音后提取正文。
    Readability,
}

/// HTTP 抓取的可选配置。
#[derive(Debug, Clone)]
pub struct FetchOptions {
    /// 请求超时秒数，默认 30。
    pub timeout_secs: u64,
    /// 返回内容的最大字符数，超过则截断，默认 30_000。
    pub max_chars: usize,
    /// 自定义 User-Agent，为空则使用默认值。
    pub user_agent: Option<String>,
}

impl Default for FetchOptions {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            max_chars: 30_000,
            user_agent: None,
        }
    }
}

/// 抓取到的网页原始数据。
#[derive(Debug, Clone)]
pub struct WebPage {
    pub url: String,
    pub title: String,
    pub html_content: String,
    pub status_code: u16,
}

/// Tavily API 等搜索引擎返回的单条结果。
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    /// Tavily 在结果层面返回的简短摘要。
    pub snippet: String,
    /// Tavily 已提取好的正文内容。
    pub content: String,
    /// 相关度评分（0.0 ~ 1.0），值越大越相关。
    pub score: f64,
}

// ─── 爬虫相关类型 ─────────────────────────────────────────────

/// 网页爬虫 BFS 配置。
/// max_chars_per_page 设得比 web_fetch 低（5000 vs 30000），
/// 因为爬虫会抓取大量页面，每页只需保留正文要点即可。
#[derive(Debug, Clone)]
pub struct CrawlConfig {
    /// 种子 URL，爬虫从此地址开始按 BFS 扩展。
    pub seed_url: String,
    /// 最大链接深度（种子页 depth=0），默认 3。
    pub max_depth: u32,
    /// 最多抓取的页面总数，默认 20。
    pub max_pages: u32,
    /// 是否只抓取与种子 URL 同域名的页面，默认 true。
    pub same_domain_only: bool,
    /// 是否遵守目标网站的 robots.txt 协议，默认 true。
    pub respect_robots_txt: bool,
    /// 连续请求之间的最小间隔（毫秒），默认 1000。
    pub request_delay_ms: u64,
    /// 单页抓取超时秒数，默认 30。
    pub timeout_secs: u64,
    /// 每页提取正文后的最大字符数，默认 5000。
    pub max_chars_per_page: usize,
}

impl Default for CrawlConfig {
    fn default() -> Self {
        Self {
            seed_url: String::new(),
            max_depth: 3,
            max_pages: 20,
            same_domain_only: true,
            respect_robots_txt: true,
            request_delay_ms: 1000,
            timeout_secs: 30,
            max_chars_per_page: 5000,
        }
    }
}

/// 爬虫抓取的单个页面结果，含提取后的纯文本正文。
#[derive(Debug, Clone, serde::Serialize)]
pub struct CrawledPage {
    pub url: String,
    pub title: String,
    pub content: String,
    /// 该页面距离种子 URL 的链接跳数。
    pub depth: u32,
}

/// 整个爬取任务的聚合结果。
#[derive(Debug, Clone, serde::Serialize)]
pub struct CrawlResult {
    pub seed_url: String,
    /// 成功抓取且提取正文的页面数量。
    pub pages_crawled: u32,
    /// 因 robots.txt、跨域、抓取失败等原因跳过的页面数量。
    pub pages_skipped: u32,
    /// 按抓取顺序排列的页面列表。
    pub pages: Vec<CrawledPage>,
    /// 逐页操作日志（OK / SKIP / FAIL），方便排查问题。
    pub logs: Vec<String>,
}