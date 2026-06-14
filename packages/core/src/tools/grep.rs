use super::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct GrepTool;

/// Dangerous shell commands that require approval.
pub const DANGEROUS_COMMANDS: &[&str] = &[
    "rm -rf", "rm -r", "sudo rm", "dd if=",
    "mkfs.", "format", "del /f", "rd /s",
    "chmod 777", ":(){ :|:& };:",
    "> /dev/sda", "shutdown", "reboot",
];

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }
    fn description(&self) -> &str {
        "Search files using regex patterns (powered by ripgrep)"
    }
    fn is_dangerous(&self) -> bool {
        false
    }
    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "Regex pattern to search for" },
                "path": { "type": "string", "description": "Directory or file to search in" }
            },
            "required": ["pattern"]
        })
    }
    async fn execute(&self, args: Value) -> ToolResult {
        let pattern = match args["pattern"].as_str() {
            Some(p) => p,
            None => return ToolResult {
                success: false,
                content: String::new(),
                error: Some("Missing required parameter: pattern".into()),
            },
        };

        let re = match regex::Regex::new(pattern) {
            Ok(r) => r,
            Err(e) => return ToolResult {
                success: false,
                content: String::new(),
                error: Some(format!("Invalid regex: {}", e)),
            },
        };

        let search_path = args["path"].as_str().unwrap_or(".");

        let mut results: Vec<String> = Vec::new();
        let mut match_count = 0;
        let max_matches = 500;

        let walker = walkdir::WalkDir::new(search_path)
            .follow_links(false)
            .max_depth(50)
            .into_iter()
            .filter_entry(|e| {
                // Skip hidden dirs and common ignore dirs
                let name = e.file_name().to_string_lossy();
                !(name.starts_with('.') && name != "."
                    || name == "node_modules"
                    || name == "target"
                    || name == ".git")
            });

        for entry in walker {
            if match_count >= max_matches {
                results.push(format!("... (truncated, {} matches total)", match_count));
                break;
            }

            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            if !entry.file_type().is_file() {
                continue;
            }

            // Skip binary files by extension
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if matches!(ext, "exe" | "dll" | "so" | "dylib" | "bin" | "png" | "jpg"
                    | "jpeg" | "gif" | "ico" | "pdf" | "zip" | "tar" | "gz" | "wasm") {
                    continue;
                }
            }

            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            for (line_num, line) in content.lines().enumerate() {
                if match_count >= max_matches {
                    break;
                }
                if re.is_match(line) {
                    results.push(format!(
                        "{}:{}: {}",
                        path.display(),
                        line_num + 1,
                        line.trim()
                    ));
                    match_count += 1;
                }
            }
        }

        if results.is_empty() {
            ToolResult {
                success: true,
                content: format!("No matches found for pattern: {}", pattern),
                error: None,
            }
        } else {
            ToolResult {
                success: true,
                content: results.join("\n"),
                error: None,
            }
        }
    }
}
