use crate::tools::{Tool, ToolOutput};
use serde_json::Value;

pub struct ReadTool;

#[async_trait::async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &str { "read" }
    fn description(&self) -> &str { "Read the contents of a file. Supports optional line offset and limit." }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the file"
                },
                "offset": {
                    "type": "integer",
                    "description": "Line number to start from (1-indexed)",
                    "default": 0
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of lines to read",
                    "default": 2000
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value) -> ToolOutput {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if path.is_empty() {
            return ToolOutput::err("path is required");
        }

        let offset = args.get("offset").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
        let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(2000) as usize;

        match tokio::fs::read_to_string(path).await {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                let start = offset.saturating_sub(1);
                let end = std::cmp::min(start + limit, lines.len());

                let excerpt: Vec<String> = lines[start..end].iter()
                    .enumerate()
                    .map(|(i, l)| format!("{}:{}", start + i + 1, l))
                    .collect();

                ToolOutput::ok(excerpt.join("\n"))
            }
            Err(e) => ToolOutput::err(format!("Failed to read file: {}", e)),
        }
    }
}
