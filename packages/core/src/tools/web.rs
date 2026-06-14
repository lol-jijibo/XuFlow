use super::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct WebFetchTool;

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }
    fn description(&self) -> &str {
        "Fetch a web page and return its text content"
    }
    fn is_dangerous(&self) -> bool {
        false
    }
    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "URL to fetch" }
            },
            "required": ["url"]
        })
    }
    async fn execute(&self, args: Value) -> ToolResult {
        let url = match args["url"].as_str() {
            Some(u) => u,
            None => return ToolResult {
                success: false,
                content: String::new(),
                error: Some("Missing required parameter: url".into()),
            },
        };

        // Validate URL scheme
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return ToolResult {
                success: false,
                content: String::new(),
                error: Some("URL must start with http:// or https://".into()),
            };
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Xuflow/0.1")
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e));

        let client = match client {
            Ok(c) => c,
            Err(e) => return ToolResult {
                success: false,
                content: String::new(),
                error: Some(e),
            },
        };

        match client.get(url).send().await {
            Ok(response) => {
                let status = response.status();
                if !status.is_success() {
                    return ToolResult {
                        success: false,
                        content: String::new(),
                        error: Some(format!("HTTP {} {}", status.as_u16(), status.canonical_reason().unwrap_or(""))),
                    };
                }

                match response.text().await {
                    Ok(html) => {
                        // Simple HTML-to-text: strip tags, normalize whitespace
                        let text = strip_html_tags(&html);
                        let truncated = if text.len() > 50_000 {
                            format!("{}...\n\n[Content truncated at 50000 characters]", &text[..50_000])
                        } else {
                            text
                        };

                        ToolResult {
                            success: true,
                            content: truncated,
                            error: None,
                        }
                    }
                    Err(e) => ToolResult {
                        success: false,
                        content: String::new(),
                        error: Some(format!("Failed to read response body: {}", e)),
                    },
                }
            }
            Err(e) => ToolResult {
                success: false,
                content: String::new(),
                error: Some(format!("HTTP request failed: {}", e)),
            },
        }
    }
}

/// Strip HTML tags and return plain text.
fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_script_style = false;
    let mut tag_name = String::new();

    for ch in html.chars() {
        if ch == '<' {
            in_tag = true;
            tag_name.clear();
        } else if ch == '>' && in_tag {
            in_tag = false;
            let lower = tag_name.to_lowercase();
            if lower == "script" || lower == "style" {
                in_script_style = true;
            } else if lower == "/script" || lower == "/style" {
                in_script_style = false;
            }
            // Add newline after block-level tags
            if matches!(lower.as_str(), "br" | "p" | "div" | "li" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "tr") {
                result.push('\n');
            }
            tag_name.clear();
        } else if in_tag {
            tag_name.push(ch);
        } else if !in_script_style {
            result.push(ch);
        }
    }

    // Decode common HTML entities
    let result = result.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");

    // Normalize whitespace: collapse multiple blank lines
    let lines: Vec<&str> = result.lines().collect();
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

    cleaned.join("\n")
}
