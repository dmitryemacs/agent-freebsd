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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Tool;

    #[tokio::test]
    async fn test_write_missing_path() {
        let tool = WriteTool;
        let result = tool.execute(serde_json::json!({"content": "hello"})).await;
        assert!(!result.success);
        assert_eq!(result.error.unwrap(), "path is required");
    }

    #[tokio::test]
    async fn test_write_success() {
        let tmp = std::env::temp_dir().join("aibsd_test_write.txt");
        let tool = WriteTool;
        let result = tool.execute(serde_json::json!({
            "path": tmp.to_str().unwrap(),
            "content": "hello world"
        })).await;
        assert!(result.success);
        assert!(result.output.contains("Wrote"));

        let content = std::fs::read_to_string(&tmp).unwrap();
        assert_eq!(content, "hello world");

        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn test_write_creates_dirs() {
        let tmp = std::env::temp_dir().join("aibsd_test_nested/sub/file.txt");
        let tool = WriteTool;
        let result = tool.execute(serde_json::json!({
            "path": tmp.to_str().unwrap(),
            "content": "nested"
        })).await;
        assert!(result.success);

        let content = std::fs::read_to_string(&tmp).unwrap();
        assert_eq!(content, "nested");

        let _ = std::fs::remove_dir_all(tmp.parent().unwrap().parent().unwrap().join("aibsd_test_nested"));
    }

    #[tokio::test]
    async fn test_write_empty_content() {
        let tmp = std::env::temp_dir().join("aibsd_test_write_empty.txt");
        let tool = WriteTool;
        let result = tool.execute(serde_json::json!({
            "path": tmp.to_str().unwrap(),
            "content": ""
        })).await;
        assert!(result.success);

        let content = std::fs::read_to_string(&tmp).unwrap();
        assert_eq!(content, "");

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_write_input_schema() {
        let tool = WriteTool;
        let schema = tool.input_schema();
        assert!(schema["required"].as_array().unwrap().iter().any(|v| v == "path"));
        assert!(schema["required"].as_array().unwrap().iter().any(|v| v == "content"));
    }
}
