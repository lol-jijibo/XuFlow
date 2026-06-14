use super::{Tool, ToolResult};
use crate::tools::grep::DANGEROUS_COMMANDS;
use async_trait::async_trait;
use serde_json::Value;

pub struct BashTool;

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }
    fn description(&self) -> &str {
        "Execute a shell command"
    }
    fn is_dangerous(&self) -> bool {
        true
    }
    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "Shell command to execute" },
                "working_dir": { "type": "string", "description": "Working directory for the command" }
            },
            "required": ["command"]
        })
    }
    async fn execute(&self, args: Value) -> ToolResult {
        let command = match args["command"].as_str() {
            Some(c) => c,
            None => return ToolResult {
                success: false,
                content: String::new(),
                error: Some("Missing required parameter: command".into()),
            },
        };

        // Check against dangerous command blacklist
        let command_lower = command.to_lowercase();
        for dangerous in DANGEROUS_COMMANDS {
            if command_lower.contains(&dangerous.to_lowercase()) {
                return ToolResult {
                    success: false,
                    content: String::new(),
                    error: Some(format!(
                        "Command blocked for safety: '{}' matches dangerous pattern '{}'",
                        command, dangerous
                    )),
                };
            }
        }

        let working_dir = args["working_dir"].as_str().unwrap_or(".");

        // Resolve shell: use cmd.exe on Windows, sh on Unix
        #[cfg(target_os = "windows")]
        let (shell, shell_arg) = ("cmd", "/C");
        #[cfg(not(target_os = "windows"))]
        let (shell, shell_arg) = ("sh", "-c");

        let output = match tokio::process::Command::new(shell)
            .arg(shell_arg)
            .arg(command)
            .current_dir(working_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
        {
            Ok(o) => o,
            Err(e) => return ToolResult {
                success: false,
                content: String::new(),
                error: Some(format!("Failed to execute command: {}", e)),
            },
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let mut content = String::new();
        if !stdout.is_empty() {
            content.push_str(&stdout);
        }
        if !stderr.is_empty() {
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str("[stderr]\n");
            content.push_str(&stderr);
        }

        if content.is_empty() {
            content = format!("Command completed with exit code: {}", output.status.code().unwrap_or(-1));
        }

        ToolResult {
            success: output.status.success(),
            content,
            error: if output.status.success() { None } else {
                Some(format!("Command exited with code: {}", output.status.code().unwrap_or(-1)))
            },
        }
    }
}
