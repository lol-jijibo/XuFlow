// 网页爬取工具：从种子 URL 出发，按 BFS 抓取同域页面并提取正文。
// 实现思路：构造 CrawlConfig 后调用 web::crawl::crawl() 执行爬取，
// 将 CrawlResult 格式化为可读的 Markdown 返回给模型。

use super::{Tool, ToolResult};
use crate::web::{CrawlConfig, CrawlResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct WebCrawlTool;

#[async_trait]
impl Tool for WebCrawlTool {
    fn name(&self) -> &str {
        "web_crawl"
    }

    fn description(&self) -> &str {
        "爬取网站，从种子 URL 出发按 BFS 抓取多个同域页面并提取正文（遵守 robots.txt）"
    }

    fn is_dangerous(&self) -> bool {
        false
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "起始种子 URL（需以 http:// 或 https:// 开头）"
                },
                "max_depth": {
                    "type": "integer",
                    "description": "最大链接深度（种子=0），默认 3，范围 0~10"
                },
                "max_pages": {
                    "type": "integer",
                    "description": "最多抓取的页面总数，默认 20，范围 1~100"
                },
                "same_domain_only": {
                    "type": "boolean",
                    "description": "是否只抓取与种子 URL 同域名的页面，默认 true。设 false 可跨域跟踪链接"
                },
                "max_chars_per_page": {
                    "type": "integer",
                    "description": "每页提取正文的最大字符数，默认 5000。爬虫场景建议低于 web_fetch 的 30000"
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let url = match args["url"].as_str() {
            Some(u) => u,
            None => {
                return ToolResult {
                    success: false,
                    content: String::new(),
                    error: Some("缺少必填参数: url".into()),
                }
            }
        };

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return ToolResult {
                success: false,
                content: String::new(),
                error: Some("URL 必须以 http:// 或 https:// 开头".into()),
            };
        }

        let max_depth = args["max_depth"]
            .as_u64()
            .map(|v| (v as u32).min(10))
            .unwrap_or(3);

        let max_pages = args["max_pages"]
            .as_u64()
            .map(|v| (v as u32).min(100))
            .unwrap_or(20);

        let same_domain_only = args["same_domain_only"].as_bool().unwrap_or(true);

        let max_chars_per_page = args["max_chars_per_page"]
            .as_u64()
            .map(|v| v as usize)
            .unwrap_or(5000);

        let config = CrawlConfig {
            seed_url: url.to_string(),
            max_depth,
            max_pages,
            same_domain_only,
            max_chars_per_page,
            ..Default::default()
        };

        // 执行爬取
        let result: CrawlResult = match crate::web::crawl::crawl(&config).await {
            Ok(r) => r,
            Err(e) => {
                return ToolResult {
                    success: false,
                    content: String::new(),
                    error: Some(format!("爬取失败: {}", e)),
                }
            }
        };

        // 格式化输出
        let output = format_crawl_result(&result);

        ToolResult {
            success: true,
            content: output,
            error: None,
        }
    }
}

/// 将 CrawlResult 格式化为可读的 Markdown，方便 LLM 理解爬取结果。
fn format_crawl_result(result: &CrawlResult) -> String {
    let mut out = String::new();

    // 摘要区
    out.push_str(&format!(
        "## 网页爬取结果\n\n\
         **种子 URL:** {}\n\
         **成功抓取:** {} 页 | **跳过/失败:** {} 页\n\n",
        result.seed_url, result.pages_crawled, result.pages_skipped,
    ));

    // 操作日志（让模型了解哪些被跳过、哪些失败）
    if !result.logs.is_empty() {
        out.push_str("### 抓取日志\n\n");
        for log in &result.logs {
            out.push_str(&format!("- {}\n", log));
        }
        out.push('\n');
    }

    // 分页正文
    if result.pages.is_empty() {
        out.push_str("### ⚠️ 未抓取到任何页面\n\n");
        out.push_str(
            "可能原因：种子 URL 不可达、所有链接被 robots.txt 拦截、超出深度/数量限制。",
        );
    } else {
        out.push_str("---\n\n");
        for (i, page) in result.pages.iter().enumerate() {
            out.push_str(&format!(
                "### 页面 {}: {} (depth={})\n\
                 **URL:** {}\n\n\
                 {}\n\n---\n\n",
                i + 1,
                if page.title.is_empty() {
                    "(无标题)"
                } else {
                    &page.title
                },
                page.depth,
                page.url,
                page.content,
            ));
        }
    }

    out
}
