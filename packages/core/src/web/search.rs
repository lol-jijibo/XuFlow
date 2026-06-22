// Tavily Search API 集成：为 AI Agent 专设的搜索引擎，一次请求同时返回搜索结果和提取好的正文。
// 免费额度 1000 次/月，适合测试场景。API Key 通过环境变量 TAVILY_API_KEY 传入。

use super::SearchResult;
use anyhow::{Context, Result};
use serde::Deserialize;

const TAVILY_SEARCH_URL: &str = "https://api.tavily.com/search";

/// Tavily search API 的原始返回结构。
#[derive(Debug, Deserialize)]
struct TavilyResponse {
    /// Tavily 为整个查询生成的 AI 摘要（仅 include_answer=true 时返回）。
    #[serde(default)]
    answer: Option<String>,
    /// 搜索结果列表，每项都包含已提取的正文。
    results: Vec<TavilyResultItem>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TavilyResultItem {
    title: String,
    url: String,
    /// 已提取好的正文内容——这是 Tavily 的核心优势，省去二次抓取。
    content: String,
    /// 简短摘要。
    #[serde(default)]
    snippet: Option<String>,
    /// 相关度评分（0.0 ～ 1.0）。
    #[serde(default)]
    score: f64,
    /// 已发布内容的日期。
    #[serde(default)]
    published_date: Option<String>,
}

/// 调用 Tavily Search API，返回带正文的搜索结果列表。
///
/// - `query`: 搜索关键词。
/// - `api_key`: Tavily API Key（从 tavily.com 免费获取）。
/// - `max_results`: 期望返回的结果条数，默认 8，最大 20。
///
/// 额外返回 `answer`（Tavily 生成的查询摘要），以 Option 形式附在结果处理中。
pub async fn tavily_search(
    query: &str,
    api_key: &str,
    max_results: u32,
) -> Result<(Option<String>, Vec<SearchResult>)> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent("Xuflow/0.1")
        .build()
        .context("创建 HTTP 客户端失败")?;

    let request_body = serde_json::json!({
        "api_key": api_key,
        "query": query,
        "search_depth": "basic",
        "max_results": max_results.min(20),
        "include_answer": true,
    });

    let response = client
        .post(TAVILY_SEARCH_URL)
        .json(&request_body)
        .send()
        .await
        .context("Tavily API 请求失败，请检查网络连接和 API Key")?;

    let status = response.status();
    if !status.is_success() {
        let error_body = response.text().await.unwrap_or_default();
        anyhow::bail!(
            "Tavily API 返回错误 HTTP {}: {}",
            status.as_u16(),
            error_body
        );
    }

    let data: TavilyResponse = response
        .json()
        .await
        .context("解析 Tavily API 返回的 JSON 失败")?;

    let results = data
        .results
        .into_iter()
        .enumerate()
        .map(|(i, item)| SearchResult {
            title: item.title,
            url: item.url,
            snippet: item.snippet.unwrap_or_else(|| {
                // 若无 snippet，用 content 的前 200 字符代替
                item.content
                    .chars()
                    .take(200)
                    .collect::<String>()
            }),
            content: item.content,
            score: if item.score > 0.0 {
                item.score
            } else {
                // 按返回顺序降权模拟评分
                1.0 - (i as f64 * 0.05)
            },
        })
        .collect();

    Ok((data.answer, results))
}