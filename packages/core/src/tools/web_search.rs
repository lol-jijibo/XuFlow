// 网页搜索工具：调用 Tavily Search API，一次请求完成搜索 + 正文提取。
// Tavily 是为 AI Agent 设计的搜索引擎，返回 JSON 结构且已包含提取好的正文。
// 通过环境变量 TAVILY_API_KEY 配置 API Key（从 tavily.com 免费注册获取）。

use super::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct WebSearchTool;

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "搜索互联网，返回带正文内容的结果列表（通过 Tavily API）"
    }

    fn is_dangerous(&self) -> bool {
        false
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "搜索关键词，支持自然语言查询"
                },
                "max_results": {
                    "type": "integer",
                    "description": "期望返回的结果条数，默认 8，最大 20"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let query = match args["query"].as_str() {
            Some(q) => q,
            None => {
                return ToolResult {
                    success: false,
                    content: String::new(),
                    error: Some("缺少必填参数: query".into()),
                }
            }
        };

        if query.trim().is_empty() {
            return ToolResult {
                success: false,
                content: String::new(),
                error: Some("搜索关键词不能为空".into()),
            };
        }

        // 从环境变量读取 API Key
        let api_key = match std::env::var("TAVILY_API_KEY") {
            Ok(k) => k,
            Err(_) => {
                return ToolResult {
                    success: false,
                    content: String::new(),
                    error: Some(
                        "未配置 TAVILY_API_KEY 环境变量。请从 https://tavily.com 注册免费 API Key 后设置。"
                            .into(),
                    ),
                }
            }
        };

        let max_results = args["max_results"]
            .as_u64()
            .map(|v| v as u32)
            .unwrap_or(8);

        // 调用 Tavily 搜索
        let (answer, results) = match crate::web::search::tavily_search(
            query, &api_key, max_results,
        )
        .await
        {
            Ok((a, r)) => (a, r),
            Err(e) => {
                return ToolResult {
                    success: false,
                    content: String::new(),
                    error: Some(format!("搜索失败: {}", e)),
                }
            }
        };

        // 格式化结果
        let mut output = format!("搜索: \"{}\"\n\n", query);

        // Tavily 的 AI 摘要放在最前面
        if let Some(ref answer_text) = answer {
            if !answer_text.is_empty() {
                output.push_str(&format!("## 摘要\n\n{}\n\n", answer_text));
            }
        }

        // 搜索结果列表
        output.push_str(&format!("## 搜索结果（共 {} 条）\n\n", results.len()));

        for (i, r) in results.iter().enumerate() {
            output.push_str(&format!(
                "### {}. {}\n\
                 - URL: {}\n\
                 - 相关度: {:.0}%\n\n\
                 {}\n\n",
                i + 1,
                r.title,
                r.url,
                r.score * 100.0,
                r.content,
            ));
        }

        if results.is_empty() {
            output.push_str("未找到相关结果。");
        }

        ToolResult {
            success: true,
            content: output,
            error: None,
        }
    }
}