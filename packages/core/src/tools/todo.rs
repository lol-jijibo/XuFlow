use super::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct TodoWriteTool;

#[async_trait]
impl Tool for TodoWriteTool {
    fn name(&self) -> &str {
        "todo_write"
    }
    fn description(&self) -> &str {
        "Create or update a structured task list to track your progress. \
         Pass the FULL list of todos each time — not just the delta. \
         Each item has: content (string), status (one of: pending, in_progress, completed). \
         Use this for multi-step tasks so the user can see what you're working on. \
         Mark the current task as 'in_progress' and update it to 'completed' when done."
    }
    fn is_dangerous(&self) -> bool {
        false
    }
    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "todos": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "content": {
                                "type": "string",
                                "description": "The task description"
                            },
                            "status": {
                                "type": "string",
                                "enum": ["pending", "in_progress", "completed"],
                                "description": "Task status"
                            }
                        },
                        "required": ["content", "status"]
                    },
                    "description": "The complete list of tasks with their current status"
                }
            },
            "required": ["todos"]
        })
    }
    async fn execute(&self, args: Value) -> ToolResult {
        // The real effect is via the agent loop emitting a TodoUpdate event.
        // Here we just validate and return the JSON so the loop can forward it.
        let todos = match args["todos"].as_array() {
            Some(arr) => arr,
            None => {
                return ToolResult {
                    success: false,
                    content: String::new(),
                    error: Some("Missing required parameter: todos (array)".into()),
                };
            }
        };

        // Validate each todo item
        for (i, item) in todos.iter().enumerate() {
            if item.get("content").and_then(|v| v.as_str()).is_none() {
                return ToolResult {
                    success: false,
                    content: String::new(),
                    error: Some(format!("Todo item {} missing 'content'", i)),
                };
            }
            let status = item.get("status").and_then(|v| v.as_str()).unwrap_or("");
            if !matches!(status, "pending" | "in_progress" | "completed") {
                return ToolResult {
                    success: false,
                    content: String::new(),
                    error: Some(format!("Todo item {} has invalid status: '{}'", i, status)),
                };
            }
        }

        let summary = serde_json::to_string(&args).unwrap_or_else(|_| String::from("[]"));
        ToolResult {
            success: true,
            content: summary,
            error: None,
        }
    }
}

pub struct ProposePlanTool;

#[async_trait]
impl Tool for ProposePlanTool {
    fn name(&self) -> &str {
        "propose_plan"
    }
    fn description(&self) -> &str {
        "Propose an implementation plan before writing code. \
         The plan must include a title, ordered list of steps, and the files that will be modified. \
         Call this tool BEFORE making changes — the user must approve the plan first."
    }
    fn is_dangerous(&self) -> bool {
        true // Needs user approval before execution proceeds
    }
    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "Short title summarizing the plan"
                },
                "steps": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Ordered implementation steps"
                },
                "files_to_modify": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Files that will be created or modified"
                }
            },
            "required": ["title", "steps", "files_to_modify"]
        })
    }
    async fn execute(&self, args: Value) -> ToolResult {
        let _title = match args["title"].as_str() {
            Some(t) => t,
            None => return ToolResult {
                success: false, content: String::new(),
                error: Some("Missing required parameter: title".into()),
            },
        };
        let _steps = match args["steps"].as_array() {
            Some(s) => s,
            None => return ToolResult {
                success: false, content: String::new(),
                error: Some("Missing required parameter: steps (array)".into()),
            },
        };
        let _files = match args["files_to_modify"].as_array() {
            Some(f) => f,
            None => return ToolResult {
                success: false, content: String::new(),
                error: Some("Missing required parameter: files_to_modify (array)".into()),
            },
        };

        // Return the plan as JSON so the agent loop can parse and emit PlanProposed
        let content = serde_json::to_string(&args).unwrap_or_else(|_| String::from("{}"));
        ToolResult {
            success: true,
            content,
            error: None,
        }
    }
}
