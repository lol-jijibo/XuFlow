// 网页抓取工具：获取指定 URL 的 HTML，智能提取正文后返回纯文本。
// 实现思路：调用 web::fetch::fetch_page 抓取页面并自动检测编码，
// 再用 web::extract::extract_content 以 Readability 模式提取正文内容。

use super::{Tool, ToolResult};
use crate::web::{self, ExtractMode, FetchOptions};
use async_trait::async_trait;
use serde_json::Value;

pub struct WebFetchTool;

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "抓取网页并提取正文文本内容（自动编码检测 + 噪音剔除）"
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
                    "description": "要抓取的网页 URL（需以 http:// 或 https:// 开头）"
                },
                "max_chars": {
                    "type": "integer",
                    "description": "返回文本的最大字符数，默认 30000"
                },
                "extract_mode": {
                    "type": "string",
                    "enum": ["readability", "text"],
                    "description": "提取模式：readability 智能定位正文（默认），text 仅去除 HTML 标签"
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

        // URL scheme 白名单校验
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return ToolResult {
                success: false,
                content: String::new(),
                error: Some("URL 必须以 http:// 或 https:// 开头".into()),
            };
        }

        let max_chars = args["max_chars"].as_u64().map(|v| v as usize).unwrap_or(30_000);

        let extract_mode = match args["extract_mode"].as_str() {
            Some("text") => ExtractMode::Text,
            _ => ExtractMode::Readability,
        };

        let opts = FetchOptions {
            max_chars,
            ..Default::default()
        };

        // 抓取页面
        let page = match web::fetch::fetch_page(url, &opts).await {
            Ok(p) => p,
            Err(e) => {
                return ToolResult {
                    success: false,
                    content: String::new(),
                    error: Some(format!("抓取页面失败: {}", e)),
                }
            }
        };

        // 提取正文
        let mut content = String::new();
        if !page.title.is_empty() {
            content.push_str(&format!("# {}\n\n", page.title));
        }
        content.push_str(&format!("URL: {}\n\n", page.url));
        content.push_str(&web::extract::extract_content(
            &page.html_content,
            extract_mode,
            max_chars,
        ));

        ToolResult {
            success: true,
            content,
            error: None,
        }
    }
}