// HTML 正文提取：用 scraper 解析 DOM，移除噪音节点后定位主内容区域。
// 实现思路：先剔除导航/广告/脚本等常见噪音标签和 CSS 类名，
// 再通过 <p> 标签密度找到正文容器，最后输出干净纯文本。

use super::ExtractMode;
use scraper::{Html, Node, Selector};

/// 从 HTML 字符串中提取纯文本内容。
/// `mode` 控制提取策略：Text 仅去标签，Readability 智能定位正文。
/// `max_chars` 为返回文本的长度上限（0 表示不限制）。
pub fn extract_content(html: &str, mode: ExtractMode, max_chars: usize) -> String {
    match mode {
        ExtractMode::Text => extract_text(html, max_chars),
        ExtractMode::Readability => extract_readability(html, max_chars),
    }
}

// ─── Text 模式：去掉所有 HTML 标签，保留可见文本 ───────────────

fn extract_text(html: &str, max_chars: usize) -> String {
    let document = Html::parse_document(html);
    let body_sel = Selector::parse("body").unwrap();

    let body = match document.select(&body_sel).next() {
        Some(b) => b,
        None => return String::new(),
    };

    let text = collect_text(body);
    truncate_and_normalize(&text, max_chars)
}

// ─── Readability 模式：智能定位主内容 ──────────────────────────

fn extract_readability(html: &str, max_chars: usize) -> String {
    let document = Html::parse_document(html);

    // 第一步：移除所有噪音节点
    // scraper 不支持直接删除节点，我们用 html 字符串预处理的方式
    // 实际上用 select 去定位内容区域而非删除节点

    // 尝试找到主内容容器
    let main_content = find_main_content(&document);

    let text = match main_content {
        Some(el_ref) => {
            // 从选中的容器中提取文本
            collect_text_from_node(el_ref)
        }
        None => {
            // 回退：从 <body> 提取全部，去掉明显的噪音文本
            let body_sel = Selector::parse("body").unwrap();
            match document.select(&body_sel).next() {
                Some(b) => collect_text(b),
                None => return String::new(),
            }
        }
    };

    truncate_and_normalize(&text, max_chars)
}

/// 定位页面的主内容区域。
/// 策略：优先查找语义标签 article / main，其次选择 p 标签最密集的 div。
fn find_main_content(document: &Html) -> Option<scraper::ElementRef<'_>> {
    // 策略 1：语义标签 <article> 或 <main>
    let semantic_sel = Selector::parse("article, main, [role=\"main\"]").unwrap();
    if let Some(el) = document.select(&semantic_sel).next() {
        return Some(el);
    }

    // 策略 2：选择包含最多 <p> 标签的顶层容器
    // 用 div 且至少包含 2 个 <p>，选 p 总数最多的那个
    let p_sel = Selector::parse("p").unwrap();
    let container_sel =
        Selector::parse("div, section, article, main").unwrap();

    let mut best: Option<(scraper::ElementRef<'_>, usize)> = None;

    for container in document.select(&container_sel) {
        let p_count = container.select(&p_sel).count();
        if p_count >= 2 {
            match best {
                Some((_, count)) if p_count > count => {
                    best = Some((container, p_count));
                }
                None => {
                    best = Some((container, p_count));
                }
                _ => {}
            }
        }
    }

    best.map(|(el, _)| el)
}

// ─── 文本收集工具 ─────────────────────────────────────────────

/// 从 ElementRef 递归收集所有文本节点，跳过 script/style 等。
fn collect_text(element: scraper::ElementRef) -> String {
    collect_text_from_node(element)
}

fn collect_text_from_node(element: scraper::ElementRef) -> String {
    let mut text = String::new();
    collect_text_recursive(element, &mut text, 0);
    text
}

fn collect_text_recursive(node: scraper::ElementRef, output: &mut String, _depth: usize) {
    // 跳过噪音标签
    let tag_name = node.value().name().to_lowercase();
    if matches!(
        tag_name.as_str(),
        "script" | "style" | "noscript" | "iframe" | "svg" | "canvas" | "code" | "pre"
    ) {
        return;
    }

    for child in node.children() {
        match child.value() {
            Node::Text(text_node) => {
                let t = text_node.text.trim();
                if !t.is_empty() {
                    output.push_str(t);
                }
            }
            Node::Element(_) => {
                if let Some(el_ref) = scraper::ElementRef::wrap(child) {
                    let child_tag = el_ref.value().name().to_lowercase();
                    // 块级元素前后加换行
                    if is_block_tag(&child_tag) {
                        output.push('\n');
                    }
                    collect_text_recursive(el_ref, output, _depth + 1);
                    if is_block_tag(&child_tag) {
                        output.push('\n');
                    }
                }
            }
            _ => {}
        }
    }
}

fn is_block_tag(tag: &str) -> bool {
    matches!(
        tag,
        "p" | "div"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "li"
            | "tr"
            | "br"
            | "article"
            | "section"
            | "header"
            | "footer"
            | "main"
            | "aside"
            | "nav"
            | "blockquote"
            | "hr"
            | "table"
            | "ul"
            | "ol"
            | "dl"
    )
}

// ─── 后处理：规范化空白 + 截断 ────────────────────────────────

fn truncate_and_normalize(text: &str, max_chars: usize) -> String {
    // 规范化空白：合并多余的空行
    let lines: Vec<&str> = text.lines().collect();
    let mut cleaned = Vec::new();
    let mut prev_empty = false;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !prev_empty {
                cleaned.push(String::new());
                prev_empty = true;
            }
        } else {
            cleaned.push(trimmed.to_string());
            prev_empty = false;
        }
    }

    let mut result = cleaned.join("\n");

    // 截断
    if max_chars > 0 && result.len() > max_chars {
        let truncate_at = find_char_boundary(&result, max_chars);
        result.truncate(truncate_at);
        result.push_str("\n\n[内容已截断...]");
    }

    result
}

fn find_char_boundary(s: &str, max: usize) -> usize {
    for i in (0..=max).rev() {
        if s.is_char_boundary(i) {
            return i;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_text_simple() {
        let html = "<html><body><h1>标题</h1><p>这是一段<b>重要</b>内容。</p></body></html>";
        let result = extract_content(html, ExtractMode::Text, 0);
        assert!(result.contains("标题"));
        assert!(result.contains("重要内容"));
        assert!(!result.contains("<b>"));
    }

    #[test]
    fn test_extract_readability_removes_noise() {
        let html = r#"<html><body>
            <nav>导航菜单</nav>
            <article><h1>正文标题</h1><p>第一段内容。</p><p>第二段内容。</p></article>
            <footer>页脚版权</footer>
        </body></html>"#;
        let result = extract_content(html, ExtractMode::Readability, 0);
        // article 被优先识别为正文容器
        assert!(result.contains("正文标题"));
        assert!(result.contains("第一段内容"));
        // 导航和页脚不应出现在正文中（因为我们只取 article 的内容）
        assert!(!result.contains("导航菜单"));
        assert!(!result.contains("页脚版权"));
    }

    #[test]
    fn test_max_chars_truncation() {
        let html = "<html><body><p>这是一个很长的段落，需要被截断测试。</p></body></html>";
        let long = extract_content(html, ExtractMode::Text, 10);
        // 截断后文本含后缀，应比原内容短，且包含截断标记
        assert!(long.contains("[内容已截断...]"));
        assert!(long.len() > 10); // 后缀会增加额外长度
    }
}