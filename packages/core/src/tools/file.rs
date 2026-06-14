use super::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct ReadFileTool;
pub struct WriteFileTool;
pub struct ListDirTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }
    fn description(&self) -> &str {
        "Read a file from the filesystem, returning its content with line numbers"
    }
    fn is_dangerous(&self) -> bool {
        false
    }
    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file to read" }
            },
            "required": ["path"]
        })
    }
    async fn execute(&self, args: Value) -> ToolResult {
        let path = match args["path"].as_str() {
            Some(p) => p,
            None => return ToolResult {
                success: false,
                content: String::new(),
                error: Some("Missing required parameter: path".into()),
            },
        };

        match tokio::fs::read_to_string(path).await {
            Ok(content) => {
                let numbered: String = content
                    .lines()
                    .enumerate()
                    .map(|(i, line)| format!("{:>6}\t{}", i + 1, line))
                    .collect::<Vec<_>>()
                    .join("\n");

                ToolResult {
                    success: true,
                    content: numbered,
                    error: None,
                }
            }
            Err(e) => ToolResult {
                success: false,
                content: String::new(),
                error: Some(format!("Failed to read file: {}", e)),
            },
        }
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }
    fn description(&self) -> &str {
        "Write or overwrite a file"
    }
    fn is_dangerous(&self) -> bool {
        true
    }
    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file to write" },
                "content": { "type": "string", "description": "Content to write" }
            },
            "required": ["path", "content"]
        })
    }
    async fn execute(&self, args: Value) -> ToolResult {
        let path = match args["path"].as_str() {
            Some(p) => p,
            None => return ToolResult {
                success: false,
                content: String::new(),
                error: Some("Missing required parameter: path".into()),
            },
        };
        let content = match args["content"].as_str() {
            Some(c) => c,
            None => return ToolResult {
                success: false,
                content: String::new(),
                error: Some("Missing required parameter: content".into()),
            },
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

        match tokio::fs::write(path, content).await {
            Ok(_) => ToolResult {
                success: true,
                content: format!("Successfully wrote {} bytes to {}", content.len(), path),
                error: None,
            },
            Err(e) => ToolResult {
                success: false,
                content: String::new(),
                error: Some(format!("Failed to write file: {}", e)),
            },
        }
    }
}

#[async_trait]
impl Tool for ListDirTool {
    fn name(&self) -> &str {
        "list_dir"
    }
    fn description(&self) -> &str {
        "List the contents of a directory"
    }
    fn is_dangerous(&self) -> bool {
        false
    }
    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Directory path" }
            },
            "required": ["path"]
        })
    }
    async fn execute(&self, args: Value) -> ToolResult {
        let path = match args["path"].as_str() {
            Some(p) => p,
            None => return ToolResult {
                success: false,
                content: String::new(),
                error: Some("Missing required parameter: path".into()),
            },
        };

        match tokio::fs::read_dir(path).await {
            Ok(mut entries) => {
                let mut items: Vec<String> = Vec::new();
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let file_type = entry.file_type().await;
                    let is_dir = file_type.map(|ft| ft.is_dir()).unwrap_or(false);
                    let prefix = if is_dir { "📁" } else { "📄" };
                    items.push(format!("{} {}", prefix, name));
                }
                items.sort();
                ToolResult {
                    success: true,
                    content: items.join("\n"),
                    error: None,
                }
            }
            Err(e) => ToolResult {
                success: false,
                content: String::new(),
                error: Some(format!("Failed to list directory: {}", e)),
            },
        }
    }
}
