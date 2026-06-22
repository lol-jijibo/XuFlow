// BFS 网页爬虫核心实现：从种子 URL 出发，按广度优先遍历同域链接，
// 抓取各页面后复用 fetch_page 和 extract_content 提取正文。
// 实现要点：URL 归一化去重、robots.txt 懒加载缓存、可配置深度/数量上限、
// 请求间延迟（避免压垮目标服务器）、同域过滤、单页失败不中断整次爬取。

use super::{CrawlConfig, CrawlResult, CrawledPage, ExtractMode, FetchOptions};
use anyhow::{Context, Result};
use scraper::Selector;
use std::collections::{HashMap, HashSet, VecDeque};
use url::Url;

/// 入口：执行一次完整的 BFS 爬取，返回包含所有已抓取页面的聚合结果。
///
/// 单页抓取/提取失败不会中断整次爬取，仅记录到 logs 并继续处理队列中的下一个 URL。
/// 仅当 seed_url 本身无法解析或 scheme 非法时才返回 Err。
pub async fn crawl(config: &CrawlConfig) -> Result<CrawlResult> {
    // 1. 解析并归一化种子 URL
    let seed_url = Url::parse(&config.seed_url)
        .with_context(|| format!("无法解析种子 URL: {}", config.seed_url))?;

    if seed_url.scheme() != "http" && seed_url.scheme() != "https" {
        anyhow::bail!("种子 URL 必须以 http:// 或 https:// 开头，当前为: {}", config.seed_url);
    }

    let seed_domain = seed_url
        .host_str()
        .unwrap_or("")
        .to_lowercase();

    // 2. 初始化数据结构
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(Url, u32)> = VecDeque::new();
    let mut pages: Vec<CrawledPage> = Vec::new();
    let mut logs: Vec<String> = Vec::new();
    let mut pages_skipped: u32 = 0;
    // robots.txt 正文缓存：domain → body（None = 无 robots.txt 或获取失败）
    let mut robots_cache: HashMap<String, Option<String>> = HashMap::new();

    // 构建可复用的 HTTP 客户端（连接池在整个爬取期间共享）
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout_secs))
        .user_agent("Xuflow/0.1 (web crawler)")
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .context("创建 HTTP 客户端失败")?;

    // 3. 种子入队
    queue.push_back((seed_url, 0));

    // 4. BFS 主循环
    while let Some((current_url, depth)) = queue.pop_front() {
        // 达到最大页数上限时终止
        if pages.len() as u32 >= config.max_pages {
            logs.push(format!("INFO 已达到最大页数限制({})，停止爬取", config.max_pages));
            break;
        }

        // 4a. 归一化后去重检查
        let normalized = normalize_url_str(&current_url);
        if visited.contains(&normalized) {
            continue;
        }

        // 4b. 深度检查
        if depth > config.max_depth {
            continue;
        }

        // 4c. 同域检查
        if config.same_domain_only {
            let current_domain = current_url
                .host_str()
                .unwrap_or("")
                .to_lowercase();
            if current_domain != seed_domain {
                logs.push(format!("SKIP [跨域] {}", current_url));
                pages_skipped += 1;
                continue;
            }
        }

        // 4d. robots.txt 检查（按域懒加载缓存正文）
        if config.respect_robots_txt {
            let domain = current_url
                .host_str()
                .unwrap_or("")
                .to_string();
            if !robots_cache.contains_key(&domain) {
                let body = fetch_robots_txt_body(
                    &client,
                    &domain,
                    current_url.scheme(),
                )
                .await;
                robots_cache.insert(domain.clone(), body);
            }

            // 若缓存中有 robots.txt 正文，用 DefaultMatcher 检查是否允许抓取
            if let Some(Some(ref body)) = robots_cache.get(&domain) {
                let mut matcher = robotstxt::DefaultMatcher::default();
                let allowed = matcher.one_agent_allowed_by_robots(
                    body,
                    "Xuflow",
                    current_url.as_str(),
                );
                if !allowed {
                    logs.push(format!("SKIP [robots.txt] {}", current_url));
                    pages_skipped += 1;
                    continue;
                }
            }
        }

        // 4e. 请求间延迟（首请求不延迟，后续请求按配置延迟）
        if !pages.is_empty() {
            tokio::time::sleep(std::time::Duration::from_millis(
                config.request_delay_ms,
            ))
            .await;
        }

        // 4f. 抓取页面（复用 web::fetch 模块）
        let fetch_opts = FetchOptions {
            timeout_secs: config.timeout_secs,
            max_chars: 0, // 不做截断，提取阶段控制长度
            user_agent: None, // 使用默认 UA
        };

        let page = match super::fetch::fetch_page(current_url.as_str(), &fetch_opts).await {
            Ok(p) => p,
            Err(e) => {
                logs.push(format!("FAIL [抓取] {} — {}", current_url, e));
                pages_skipped += 1;
                continue;
            }
        };

        // 4g. 处理重定向：标记原始 URL 和最终 URL 为已访问
        visited.insert(normalized);
        let final_url = Url::parse(&page.url).ok();
        if let Some(ref final_parsed) = final_url {
            let final_normalized = normalize_url_str(final_parsed);
            if final_normalized != normalize_url_str(&current_url) {
                if visited.contains(&final_normalized) {
                    // 重定向到已抓取过的页面，跳过
                    logs.push(format!(
                        "SKIP [重定向去重] {} → {}",
                        current_url, page.url
                    ));
                    pages_skipped += 1;
                    continue;
                }
                visited.insert(final_normalized);
            }
        }

        // 4h. 提取正文（复用 web::extract 模块的 Readability 模式）
        let content = super::extract::extract_content(
            &page.html_content,
            ExtractMode::Readability,
            config.max_chars_per_page,
        );

        pages.push(CrawledPage {
            url: page.url.clone(),
            title: page.title.clone(),
            content,
            depth,
        });

        logs.push(format!(
            "OK [depth={}] {} — {}",
            depth,
            if page.title.is_empty() {
                "(无标题)"
            } else {
                &page.title
            },
            page.url,
        ));

        // 4i. 提取子链接并入队（仅当未达到最大深度时）
        if depth < config.max_depth {
            let base_for_links =
                final_url.unwrap_or_else(|| current_url.clone());
            let child_links = extract_links(
                &page.html_content,
                &base_for_links,
                &seed_domain,
                config.same_domain_only,
            );

            for child_url in child_links {
                let child_normalized = normalize_url_str(&child_url);
                if !visited.contains(&child_normalized) {
                    queue.push_back((child_url, depth + 1));
                }
            }
        }
    }

    Ok(CrawlResult {
        seed_url: config.seed_url.clone(),
        pages_crawled: pages.len() as u32,
        pages_skipped,
        pages,
        logs,
    })
}

// ─── URL 归一化 ─────────────────────────────────────────────────

/// 归一化 URL 字符串用于去重比较：
/// 去除 fragment（#anchor）、去除路径尾随斜杠（保留 /）、全小写。
fn normalize_url_str(url: &Url) -> String {
    let mut u = url.clone();
    u.set_fragment(None);
    // 规范化 path 尾斜杠：/path/ → /path，但根路径 / 保持不变
    let path = u.path().to_string();
    if path.len() > 1 && path.ends_with('/') {
        u.set_path(&path[..path.len() - 1]);
    }
    u.as_str().to_lowercase()
}

// ─── 链接提取 ──────────────────────────────────────────────────

/// 从 HTML 中提取所有 `<a href="...">` 链接，解析相对 URL，过滤非 http/https。
/// 返回归一化后的绝对 URL 列表。
fn extract_links(
    html: &str,
    base_url: &Url,
    seed_domain: &str,
    same_domain_only: bool,
) -> Vec<Url> {
    let document = scraper::Html::parse_document(html);
    let a_selector = Selector::parse("a[href]").unwrap();

    let mut links = Vec::new();

    for element in document.select(&a_selector) {
        if let Some(href) = element.value().attr("href") {
            // 跳过空链接、仅 fragment 链接、javascript:伪协议
            let trimmed = href.trim();
            if trimmed.is_empty()
                || trimmed.starts_with('#')
                || trimmed.starts_with("javascript:")
                || trimmed.starts_with("mailto:")
                || trimmed.starts_with("tel:")
            {
                continue;
            }

            // 解析相对 URL，失败则跳过
            let resolved = match base_url.join(trimmed) {
                Ok(u) => u,
                Err(_) => continue,
            };

            // 仅接受 http/https
            if resolved.scheme() != "http" && resolved.scheme() != "https" {
                continue;
            }

            // 可选同域过滤
            if same_domain_only {
                let domain = resolved.host_str().unwrap_or("").to_lowercase();
                if domain != seed_domain {
                    continue;
                }
            }

            // 去 fragment 后加入结果
            let mut clean = resolved.clone();
            clean.set_fragment(None);
            links.push(clean);
        }
    }

    links
}

// ─── robots.txt 集成 ───────────────────────────────────────────

/// 抓取指定域的 robots.txt 正文。
/// 返回 None 表示抓取失败或文件不存在（视为"允许全部"）。
async fn fetch_robots_txt_body(
    client: &reqwest::Client,
    domain: &str,
    scheme: &str,
) -> Option<String> {
    let robots_url = format!("{}://{}/robots.txt", scheme, domain);

    let resp = match client.get(&robots_url).send().await {
        Ok(r) => r,
        Err(_) => return None, // 网络错误，允许全部
    };

    if !resp.status().is_success() {
        return None; // 404 等，视为无 robots.txt
    }

    resp.text().await.ok() // 读取失败也视为允许全部
}

// ─── 单元测试 ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_url_strips_fragment() {
        let url = Url::parse("https://example.com/page?q=1#section").unwrap();
        let normalized = normalize_url_str(&url);
        assert!(!normalized.contains('#'));
        assert!(normalized.contains("?q=1"));
    }

    #[test]
    fn test_normalize_url_strips_trailing_slash() {
        let url = Url::parse("https://example.com/path/").unwrap();
        let normalized = normalize_url_str(&url);
        assert_eq!(normalized, "https://example.com/path");
    }

    #[test]
    fn test_normalize_url_preserves_root_slash() {
        let url = Url::parse("https://example.com/").unwrap();
        let normalized = normalize_url_str(&url);
        assert_eq!(normalized, "https://example.com/");
    }

    #[test]
    fn test_extract_links_resolves_relative() {
        let base = Url::parse("https://example.com/blog/").unwrap();
        let html = r#"<html><body>
            <a href="/about">About</a>
            <a href="post-1.html">Post 1</a>
        </body></html>"#;
        let links = extract_links(html, &base, "example.com", true);
        let urls: Vec<String> = links.iter().map(|u| u.as_str().to_string()).collect();
        assert!(urls.contains(&"https://example.com/about".to_string()));
        assert!(urls.contains(&"https://example.com/blog/post-1.html".to_string()));
    }

    #[test]
    fn test_extract_links_filters_javascript() {
        let base = Url::parse("https://example.com/").unwrap();
        let html = r#"<a href="javascript:void(0)">click</a><a href="/real">Real</a>"#;
        let links = extract_links(html, &base, "example.com", true);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].as_str(), "https://example.com/real");
    }

    #[test]
    fn test_extract_links_filters_cross_domain() {
        let base = Url::parse("https://example.com/").unwrap();
        let html = r#"<a href="https://other.com/page">Other</a><a href="/local">Local</a>"#;
        let links = extract_links(html, &base, "example.com", true);
        assert_eq!(links.len(), 1);
        assert!(links[0].as_str().contains("example.com"));
    }
}
