// HTTP 抓取实现：发起 GET 请求，自动检测字符编码，解码为 UTF-8 字符串。
// 通过 encoding_rs 解决中文等非 UTF-8 页面的乱码问题。

use super::{FetchOptions, WebPage};
use anyhow::{Context, Result};
use encoding_rs::Encoding;
use scraper::Html;

/// 抓取指定 URL，返回包含解码后 HTML 的 WebPage。
/// 自动跟随重定向（最多 10 跳），通过 Content-Type header 或 encoding_rs 检测编码。
pub async fn fetch_page(url: &str, opts: &FetchOptions) -> Result<WebPage> {
    let user_agent = opts
        .user_agent
        .as_deref()
        .unwrap_or("Xuflow/0.1 (AI agent)");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(opts.timeout_secs))
        .user_agent(user_agent)
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .context("创建 HTTP 客户端失败")?;

    let response = client
        .get(url)
        .send()
        .await
        .context("HTTP 请求失败")?;

    let status_code = response.status().as_u16();
    let final_url = response.url().to_string();

    if !response.status().is_success() {
        anyhow::bail!(
            "HTTP {} {}",
            status_code,
            response.status().canonical_reason().unwrap_or("未知状态码")
        );
    }

    // 检查 Content-Type，只处理 HTML 页面；提前转为 owned String 避免借用冲突
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    if !content_type.is_empty()
        && !content_type.contains("text/html")
        && !content_type.contains("text/plain")
        && !content_type.contains("application/xhtml")
    {
        anyhow::bail!(
            "不支持的内容类型: {}. 仅处理 HTML 页面。",
            content_type
        );
    }

    // 读取原始字节，不要直接 .text()，以便自己做编码检测
    let body_bytes = response.bytes().await.context("读取响应体失败")?;

    // 编码检测：优先从 Content-Type header 取 charset，否则用 encoding_rs 猜测
    let body_str = decode_html(&body_bytes, &content_type)?;

    // 提取页面 title
    let title = Html::parse_document(&body_str)
        .select(&scraper::Selector::parse("title").unwrap())
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string())
        .unwrap_or_default();

    Ok(WebPage {
        url: final_url,
        title,
        html_content: body_str,
        status_code,
    })
}

/// 根据 Content-Type header 的 charset 信息解码字节流。
///
/// 检测优先级：
/// 1. Content-Type header 中声明的 charset（最权威）
/// 2. BOM（UTF-8 / UTF-16LE / UTF-16BE）
/// 3. 尝试 UTF-8 解码，若合法则采用（现代网页 >95% 是 UTF-8）
/// 4. 若 UTF-8 失败，用 encoding_rs 的统计检测猜测编码（GBK/Shift_JIS/EUC-KR 等）
/// 5. 最后兜底：用替代字符（U+FFFD）强制解码
fn decode_html(bytes: &[u8], content_type: &str) -> Result<String> {
    // 第一步：从 Content-Type header 提取 charset 声明
    let header_encoding = content_type
        .split(';')
        .skip(1)
        .find_map(|part| {
            let kv: Vec<&str> = part.splitn(2, '=').collect();
            if kv.len() == 2 && kv[0].trim().eq_ignore_ascii_case("charset") {
                Some(kv[1].trim().trim_matches('"').trim_matches('\'').to_string())
            } else {
                None
            }
        });

    // 如果 header 声明了编码，直接按声明解码
    if let Some(ref charset) = header_encoding {
        if let Some(enc) = Encoding::for_label(charset.as_bytes()) {
            let (decoded, _, _) = enc.decode(bytes);
            return Ok(decoded.into_owned());
        }
    }

    // 第二步：检查 BOM（UTF-8 EF BB BF，UTF-16LE FF FE，UTF-16BE FE FF）
    if let Some((enc, _)) = Encoding::for_bom(bytes) {
        let (decoded, _, _) = enc.decode(bytes);
        return Ok(decoded.into_owned());
    }

    // 第三步：尝试 UTF-8 严格解码（覆盖绝大多数现代网页）
    if let Ok(s) = std::str::from_utf8(bytes) {
        return Ok(s.to_string());
    }

    // 第四步：UTF-8 不合法，用 encoding_rs 的统计检测
    // encoding_rs 不内置统计检测，但提供了常见非 UTF-8 编码的 label 匹配。
    // 对中文网页最可能的是 GBK/GB2312，日文是 Shift_JIS，韩文是 EUC-KR。
    let legacy_encodings = ["gbk", "gb2312", "gb18030", "shift_jis", "euc-jp", "euc-kr", "big5"];

    for label in legacy_encodings {
        if let Some(enc) = Encoding::for_label_no_replacement(label.as_bytes()) {
            let (decoded, _, had_errors) = enc.decode(bytes);
            if !had_errors {
                return Ok(decoded.into_owned());
            }
        }
    }

    // 第五步：兜底——用 GBK 解码（覆盖中文互联网的绝大多数遗留页面）
    // GBK 是 GB2312 的超集，几乎任何字节序列都能合法解码
    if let Some(enc) = Encoding::for_label("gbk".as_bytes()) {
        let (decoded, _, _) = enc.decode(bytes);
        return Ok(decoded.into_owned());
    }

    // 最后最后：无法处理，报错
    anyhow::bail!("无法检测页面编码，请手动指定 charset")
}