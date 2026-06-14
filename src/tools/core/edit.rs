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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Tool;

    #[tokio::test]
    async fn test_edit_missing_params() {
        let tool = EditTool;
        let result = tool.execute(serde_json::json!({})).await;
        assert!(!result.success);

        let result2 = tool.execute(serde_json::json!({"file_path": "/tmp/x"})).await;
        assert!(!result2.success);
    }

    #[tokio::test]
    async fn test_edit_nonexistent_file() {
        let tool = EditTool;
        let result = tool.execute(serde_json::json!({
            "file_path": "/tmp/nonexistent-edit-12345",
            "old_string": "foo",
            "new_string": "bar"
        })).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Failed to read file"));
    }

    #[tokio::test]
    async fn test_edit_old_string_not_found() {
        let tmp = std::env::temp_dir().join("aibsd_test_edit_nomatch.txt");
        std::fs::write(&tmp, "hello world").unwrap();

        let tool = EditTool;
        let result = tool.execute(serde_json::json!({
            "file_path": tmp.to_str().unwrap(),
            "old_string": "zzz",
            "new_string": "aaa"
        })).await;
        assert!(!result.success);
        assert_eq!(result.error.unwrap(), "old_string not found in file");

        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn test_edit_success() {
        let tmp = std::env::temp_dir().join("aibsd_test_edit_ok.txt");
        std::fs::write(&tmp, "hello world foo").unwrap();

        let tool = EditTool;
        let result = tool.execute(serde_json::json!({
            "file_path": tmp.to_str().unwrap(),
            "old_string": "foo",
            "new_string": "bar"
        })).await;
        assert!(result.success);

        let content = std::fs::read_to_string(&tmp).unwrap();
        assert_eq!(content, "hello world bar");

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_edit_input_schema() {
        let tool = EditTool;
        let schema = tool.input_schema();
        assert!(schema["required"].as_array().unwrap().iter().any(|v| v == "file_path"));
        assert!(schema["required"].as_array().unwrap().iter().any(|v| v == "old_string"));
    }
}
