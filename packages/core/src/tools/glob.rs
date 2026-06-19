use super::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct GlobTool;

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }
    fn description(&self) -> &str {
        "Find files matching a glob pattern (e.g. '**/*.rs', 'src/**/*.vue'). \
         Returns sorted file paths, skipping hidden dirs, node_modules, target, and .git."
    }
    fn is_dangerous(&self) -> bool {
        false
    }
    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern to match files, e.g. '**/*.ts' or 'src/components/**/*.vue'"
                },
                "path": {
                    "type": "string",
                    "description": "Base directory to search from (defaults to current working directory)"
                }
            },
            "required": ["pattern"]
        })
    }
    async fn execute(&self, args: Value) -> ToolResult {
        let pattern = match args["pattern"].as_str() {
            Some(p) => p,
            None => {
                return ToolResult {
                    success: false,
                    content: String::new(),
                    error: Some("Missing required parameter: pattern".into()),
                };
            }
        };

        let base_path = args["path"].as_str().unwrap_or(".");

        // Build the full glob pattern: base_path/pattern
        let full_pattern = if base_path == "." {
            pattern.to_string()
        } else {
            format!("{}/{}", base_path.trim_end_matches('/'), pattern.trim_start_matches('/'))
        };

        let mut results: Vec<String> = Vec::new();
        let max_results = 1000;

        match glob::glob(&full_pattern) {
            Ok(paths) => {
                for entry in paths {
                    if results.len() >= max_results {
                        results.push(format!(
                            "... (truncated, {} results total — narrow your pattern)",
                            max_results
                        ));
                        break;
                    }

                    let path = match entry {
                        Ok(p) => p,
                        Err(_) => continue,
                    };

                    // Only include files (skip directories)
                    if !path.is_file() {
                        continue;
                    }

                    let path_str = path.display().to_string();

                    // Skip hidden files/dirs, node_modules, target, .git
                    if path_str.contains("/.") || path_str.contains("\\.")
                        || path_str.contains("/node_modules/") || path_str.contains("\\node_modules\\")
                        || path_str.contains("/target/") || path_str.contains("\\target\\")
                    {
                        continue;
                    }

                    results.push(path_str);
                }
            }
            Err(e) => {
                return ToolResult {
                    success: false,
                    content: String::new(),
                    error: Some(format!("Invalid glob pattern: {}", e)),
                };
            }
        }

        results.sort();

        if results.is_empty() {
            ToolResult {
                success: true,
                content: format!("No files found matching pattern: {}", full_pattern),
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
