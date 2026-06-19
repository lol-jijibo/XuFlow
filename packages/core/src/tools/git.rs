use super::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Run a git command and return stdout + stderr + exit code.
async fn run_git_command(
    working_dir: &str,
    args: &[&str],
) -> Result<(String, String, i32), String> {
    let output = tokio::process::Command::new("git")
        .args(args)
        .current_dir(working_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to execute git: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);

    Ok((stdout, stderr, code))
}

/// Validate that a working_dir exists and is a directory.
fn validate_dir(path: &str) -> Result<(), String> {
    let p = std::path::Path::new(path);
    if !p.exists() {
        Err(format!("Directory does not exist: {}", path))
    } else if !p.is_dir() {
        Err(format!("Path is not a directory: {}", path))
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// git_status
// ---------------------------------------------------------------------------

pub struct GitStatusTool;

#[async_trait]
impl Tool for GitStatusTool {
    fn name(&self) -> &str {
        "git_status"
    }
    fn description(&self) -> &str {
        "Show the working tree status (git status --porcelain)."
    }
    fn is_dangerous(&self) -> bool {
        false
    }
    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "working_dir": {
                    "type": "string",
                    "description": "The git repository directory (defaults to '.')"
                }
            },
            "required": []
        })
    }
    async fn execute(&self, args: Value) -> ToolResult {
        let dir = args["working_dir"].as_str().unwrap_or(".");
        if let Err(e) = validate_dir(dir) {
            return ToolResult { success: false, content: String::new(), error: Some(e) };
        }
        match run_git_command(dir, &["status", "--porcelain"]).await {
            Ok((stdout, stderr, code)) => {
                let content = if stdout.is_empty() && stderr.is_empty() {
                    "Working tree clean — no changes.".into()
                } else if stdout.is_empty() {
                    stderr
                } else {
                    stdout
                };
                ToolResult {
                    success: code == 0,
                    content,
                    error: if code != 0 { Some(format!("git status exited with code {}", code)) } else { None },
                }
            }
            Err(e) => ToolResult { success: false, content: String::new(), error: Some(e) },
        }
    }
}

// ---------------------------------------------------------------------------
// git_diff
// ---------------------------------------------------------------------------

pub struct GitDiffTool;

#[async_trait]
impl Tool for GitDiffTool {
    fn name(&self) -> &str {
        "git_diff"
    }
    fn description(&self) -> &str {
        "Show changes in the working directory (git diff). Use staged=true for staged changes."
    }
    fn is_dangerous(&self) -> bool {
        false
    }
    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "working_dir": {
                    "type": "string",
                    "description": "The git repository directory (defaults to '.')"
                },
                "staged": {
                    "type": "boolean",
                    "description": "If true, show staged changes (git diff --staged)"
                }
            },
            "required": []
        })
    }
    async fn execute(&self, args: Value) -> ToolResult {
        let dir = args["working_dir"].as_str().unwrap_or(".");
        if let Err(e) = validate_dir(dir) {
            return ToolResult { success: false, content: String::new(), error: Some(e) };
        }
        let staged = args["staged"].as_bool().unwrap_or(false);
        let mut cmd_args = vec!["diff"];
        if staged {
            cmd_args.push("--staged");
        }
        match run_git_command(dir, &cmd_args).await {
            Ok((stdout, _stderr, code)) => {
                let content = if stdout.is_empty() {
                    "No changes.".into()
                } else {
                    // Truncate at 50000 chars to avoid overflowing context
                    if stdout.len() > 50_000 {
                        format!("{}...\n\n[Diff truncated at 50000 characters]", &stdout[..50_000])
                    } else {
                        stdout
                    }
                };
                ToolResult {
                    success: code == 0,
                    content,
                    error: if code != 0 { Some(format!("git diff exited with code {}", code)) } else { None },
                }
            }
            Err(e) => ToolResult { success: false, content: String::new(), error: Some(e) },
        }
    }
}

// ---------------------------------------------------------------------------
// git_log
// ---------------------------------------------------------------------------

pub struct GitLogTool;

#[async_trait]
impl Tool for GitLogTool {
    fn name(&self) -> &str {
        "git_log"
    }
    fn description(&self) -> &str {
        "Show recent commit history (git log --oneline)."
    }
    fn is_dangerous(&self) -> bool {
        false
    }
    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "working_dir": {
                    "type": "string",
                    "description": "The git repository directory (defaults to '.')"
                },
                "count": {
                    "type": "integer",
                    "description": "Number of recent commits to show (default: 10)"
                }
            },
            "required": []
        })
    }
    async fn execute(&self, args: Value) -> ToolResult {
        let dir = args["working_dir"].as_str().unwrap_or(".");
        if let Err(e) = validate_dir(dir) {
            return ToolResult { success: false, content: String::new(), error: Some(e) };
        }
        let count = args["count"].as_u64().unwrap_or(10).to_string();
        match run_git_command(dir, &["log", "--oneline", "-n", &count]).await {
            Ok((stdout, _, code)) => {
                let content = if stdout.is_empty() {
                    "No commits yet.".into()
                } else {
                    stdout
                };
                ToolResult {
                    success: code == 0,
                    content,
                    error: if code != 0 { Some(format!("git log exited with code {}", code)) } else { None },
                }
            }
            Err(e) => ToolResult { success: false, content: String::new(), error: Some(e) },
        }
    }
}

// ---------------------------------------------------------------------------
// git_add
// ---------------------------------------------------------------------------

pub struct GitAddTool;

#[async_trait]
impl Tool for GitAddTool {
    fn name(&self) -> &str {
        "git_add"
    }
    fn description(&self) -> &str {
        "Stage files for commit (git add <files>)."
    }
    fn is_dangerous(&self) -> bool {
        true
    }
    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "working_dir": {
                    "type": "string",
                    "description": "The git repository directory (defaults to '.')"
                },
                "files": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of file paths to stage"
                }
            },
            "required": ["files"]
        })
    }
    async fn execute(&self, args: Value) -> ToolResult {
        let dir = args["working_dir"].as_str().unwrap_or(".");
        if let Err(e) = validate_dir(dir) {
            return ToolResult { success: false, content: String::new(), error: Some(e) };
        }

        let files: Vec<&str> = match args["files"].as_array() {
            Some(arr) => arr.iter().filter_map(|v| v.as_str()).collect(),
            None => {
                return ToolResult {
                    success: false, content: String::new(),
                    error: Some("Missing required parameter: files (array of file paths)".into()),
                };
            }
        };

        if files.is_empty() {
            return ToolResult {
                success: false, content: String::new(),
                error: Some("files array is empty".into()),
            };
        }

        let mut cmd_args = vec!["add"];
        for f in &files {
            cmd_args.push(f);
        }

        match run_git_command(dir, &cmd_args).await {
            Ok((stdout, stderr, code)) => {
                let mut content = String::new();
                if !stdout.is_empty() {
                    content.push_str(&stdout);
                }
                if !stderr.is_empty() {
                    if !content.is_empty() { content.push('\n'); }
                    content.push_str(&stderr);
                }
                if content.is_empty() {
                    content = format!("Staged {} file(s).", files.len());
                }
                ToolResult {
                    success: code == 0,
                    content,
                    error: if code != 0 { Some(format!("git add exited with code {}", code)) } else { None },
                }
            }
            Err(e) => ToolResult { success: false, content: String::new(), error: Some(e) },
        }
    }
}

// ---------------------------------------------------------------------------
// git_commit
// ---------------------------------------------------------------------------

pub struct GitCommitTool;

#[async_trait]
impl Tool for GitCommitTool {
    fn name(&self) -> &str {
        "git_commit"
    }
    fn description(&self) -> &str {
        "Commit staged changes (git commit -m <message>). Only commit what was staged — never add+commit in one step."
    }
    fn is_dangerous(&self) -> bool {
        true
    }
    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "working_dir": {
                    "type": "string",
                    "description": "The git repository directory (defaults to '.')"
                },
                "message": {
                    "type": "string",
                    "description": "The commit message"
                }
            },
            "required": ["message"]
        })
    }
    async fn execute(&self, args: Value) -> ToolResult {
        let dir = args["working_dir"].as_str().unwrap_or(".");
        if let Err(e) = validate_dir(dir) {
            return ToolResult { success: false, content: String::new(), error: Some(e) };
        }
        let message = match args["message"].as_str() {
            Some(m) => m,
            None => {
                return ToolResult {
                    success: false, content: String::new(),
                    error: Some("Missing required parameter: message".into()),
                };
            }
        };

        match run_git_command(dir, &["commit", "-m", message]).await {
            Ok((stdout, stderr, code)) => {
                let mut content = String::new();
                if !stdout.is_empty() {
                    content.push_str(&stdout);
                }
                if !stderr.is_empty() {
                    if !content.is_empty() { content.push('\n'); }
                    content.push_str(&stderr);
                }
                if content.is_empty() {
                    content = format!("Committed with message: {}", message);
                }
                ToolResult {
                    success: code == 0,
                    content,
                    error: if code != 0 { Some(format!("git commit exited with code {}", code)) } else { None },
                }
            }
            Err(e) => ToolResult { success: false, content: String::new(), error: Some(e) },
        }
    }
}
