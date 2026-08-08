use super::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct EditFileTool;

#[async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &str {
        "edit"
    }
    fn description(&self) -> &str {
        "Make a precise string replacement in a file. \
         The old_string must match exactly one location (or use replace_all=true for global replace). \
         If old_string is not found or matches multiple locations, the edit is rejected with details \
         so you can retry with a more specific string."
    }
    fn is_dangerous(&self) -> bool {
        true
    }
    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to edit"
                },
                "old_string": {
                    "type": "string",
                    "description": "The exact text to replace. Must match exactly one location in the file (or use replace_all for global replacement)."
                },
                "new_string": {
                    "type": "string",
                    "description": "The text to replace old_string with."
                },
                "replace_all": {
                    "type": "boolean",
                    "description": "If true, replace ALL occurrences of old_string (default: false)."
                }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }
    async fn execute(&self, args: Value) -> ToolResult {
        let path = match args["path"].as_str() {
            Some(p) => p,
            None => {
                return ToolResult {
                    success: false,
                    content: String::new(),
                    error: Some("Missing required parameter: path".into()),
                };
            }
        };
        let old_string = match args["old_string"].as_str() {
            Some(s) => s,
            None => {
                return ToolResult {
                    success: false,
                    content: String::new(),
                    error: Some("Missing required parameter: old_string".into()),
                };
            }
        };
        let new_string = match args["new_string"].as_str() {
            Some(s) => s,
            None => {
                return ToolResult {
                    success: false,
                    content: String::new(),
                    error: Some("Missing required parameter: new_string".into()),
                };
            }
        };
        let replace_all = args["replace_all"].as_bool().unwrap_or(false);

        // Read the file
        let content = match tokio::fs::read_to_string(path).await {
            Ok(c) => c,
            Err(e) => {
                return ToolResult {
                    success: false,
                    content: String::new(),
                    error: Some(format!("Failed to read file '{}': {}", path, e)),
                };
            }
        };

        // Count occurrences of old_string
        let matches: Vec<(usize, &str)> = content
            .lines()
            .enumerate()
            .filter(|(_, line)| line.contains(old_string))
            .collect();

        let occurrence_count = content.matches(old_string).count();

        if occurrence_count == 0 {
            return ToolResult {
                success: false,
                content: String::new(),
                error: Some(format!(
                    "old_string not found in '{}'. The string you provided does not appear in the file. \
                     Re-read the file to verify the exact text to replace.",
                    path
                )),
            };
        }

        if occurrence_count > 1 && !replace_all {
            // Report all matching locations so the model can narrow down
            let location_info: Vec<String> = matches
                .iter()
                .map(|(line_num, line)| {
                    format!("  Line {}: {}", line_num + 1, line.trim())
                })
                .collect();

            return ToolResult {
                success: false,
                content: String::new(),
                error: Some(format!(
                    "old_string matches {} locations in '{}'. Use replace_all=true for global replacement, \
                     or provide a more specific string (include more surrounding context) to match exactly one location.\n\n\
                     Matching locations:\n{}",
                    occurrence_count,
                    path,
                    location_info.join("\n")
                )),
            };
        }

        // Perform the replacement
        let new_content = if replace_all {
            content.replace(old_string, new_string)
        } else {
            content.replacen(old_string, new_string, 1)
        };

        // Ensure parent directory exists
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                if let Err(e) = tokio::fs::create_dir_all(parent).await {
                    return ToolResult {
                        success: false,
                        content: String::new(),
                        error: Some(format!("Failed to create parent directory: {}", e)),
                    };
                }
            }
        }

        // Write back
        match tokio::fs::write(path, &new_content).await {
            Ok(_) => {
                let replaced_count = if replace_all { occurrence_count } else { 1 };
                ToolResult {
                    success: true,
                    content: format!(
                        "Successfully edited '{}': {} replacement(s) made.",
                        path, replaced_count
                    ),
                    error: None,
                }
            }
            Err(e) => ToolResult {
                success: false,
                content: String::new(),
                error: Some(format!("Failed to write file '{}': {}", path, e)),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::fs;

    async fn create_temp_file(content: &str) -> (String, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, content).await.unwrap();
        (file_path.to_string_lossy().to_string(), dir)
    }

    #[tokio::test]
    async fn test_edit_single_replacement() {
        let (path, _dir) = create_temp_file("hello world\nfoo bar\n").await;
        let tool = EditFileTool;
        let result = tool.execute(serde_json::json!({
            "path": path,
            "old_string": "hello world",
            "new_string": "hi there"
        })).await;
        assert!(result.success, "edit failed: {:?}", result.error);
        let content = fs::read_to_string(&path).await.unwrap();
        assert_eq!(content, "hi there\nfoo bar\n");
    }

    #[tokio::test]
    async fn test_edit_not_found() {
        let (path, _dir) = create_temp_file("hello world\n").await;
        let tool = EditFileTool;
        let result = tool.execute(serde_json::json!({
            "path": path,
            "old_string": "nonexistent",
            "new_string": "xxx"
        })).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("not found"));
    }

    #[tokio::test]
    async fn test_edit_multiple_matches_no_replace_all() {
        let (path, _dir) = create_temp_file("dup\ncontent\ndup\n").await;
        let tool = EditFileTool;
        let result = tool.execute(serde_json::json!({
            "path": path,
            "old_string": "dup",
            "new_string": "xxx",
            "replace_all": false
        })).await;
        assert!(!result.success);
        let err = result.error.unwrap();
        assert!(err.contains("matches 2 locations"), "got: {err}");
    }

    #[tokio::test]
    async fn test_edit_replace_all() {
        let (path, _dir) = create_temp_file("dup\ncontent\ndup\n").await;
        let tool = EditFileTool;
        let result = tool.execute(serde_json::json!({
            "path": path,
            "old_string": "dup",
            "new_string": "xxx",
            "replace_all": true
        })).await;
        assert!(result.success, "edit failed: {:?}", result.error);
        let content = fs::read_to_string(&path).await.unwrap();
        assert_eq!(content, "xxx\ncontent\nxxx\n");
    }

    #[tokio::test]
    async fn test_edit_file_missing() {
        let tool = EditFileTool;
        let result = tool.execute(serde_json::json!({
            "path": "/nonexistent/path/file.txt",
            "old_string": "x",
            "new_string": "y"
        })).await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_edit_missing_parameters() {
        let tool = EditFileTool;
        let result = tool.execute(serde_json::json!({})).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Missing"));
    }
}
