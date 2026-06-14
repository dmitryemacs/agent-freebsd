use crate::tools::{Tool, ToolOutput};
use serde_json::Value;

pub struct EditTool;

#[async_trait::async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str { "edit" }
    fn description(&self) -> &str { "Edit a file by replacing exact string matches. Uses search-and-replace within a specific file." }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Absolute path to the file"
                },
                "old_string": {
                    "type": "string",
                    "description": "Text to search for (exact match)"
                },
                "new_string": {
                    "type": "string",
                    "description": "Text to replace with"
                }
            },
            "required": ["file_path", "old_string", "new_string"]
        })
    }

    async fn execute(&self, args: Value) -> ToolOutput {
        let file_path = args.get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let old_string = args.get("old_string")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let new_string = args.get("new_string")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if file_path.is_empty() || old_string.is_empty() {
            return ToolOutput::err("file_path and old_string are required");
        }

        match tokio::fs::read_to_string(file_path).await {
            Ok(content) => {
                if !content.contains(old_string) {
                    return ToolOutput::err("old_string not found in file");
                }
                let new_content = content.replace(old_string, new_string);
                match tokio::fs::write(file_path, new_content).await {
                    Ok(_) => ToolOutput::ok("File edited successfully"),
                    Err(e) => ToolOutput::err(format!("Failed to write file: {}", e)),
                }
            }
            Err(e) => ToolOutput::err(format!("Failed to read file: {}", e)),
        }
    }
}
