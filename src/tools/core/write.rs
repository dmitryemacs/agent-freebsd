use crate::tools::{Tool, ToolOutput};
use serde_json::Value;

pub struct WriteTool;

#[async_trait::async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &str { "write" }
    fn description(&self) -> &str { "Write content to a file, creating or overwriting it." }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the file"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, args: Value) -> ToolOutput {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let content = args.get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if path.is_empty() {
            return ToolOutput::err("path is required");
        }

        // Ensure parent directory exists
        if let Some(parent) = std::path::Path::new(path).parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        match tokio::fs::write(path, content).await {
            Ok(_) => ToolOutput::ok(format!("Wrote {} bytes to {}", content.len(), path)),
            Err(e) => ToolOutput::err(format!("Failed to write file: {}", e)),
        }
    }
}
