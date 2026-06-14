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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Tool;

    #[tokio::test]
    async fn test_read_missing_path() {
        let tool = ReadTool;
        let result = tool.execute(serde_json::json!({})).await;
        assert!(!result.success);
        assert_eq!(result.error.unwrap(), "path is required");
    }

    #[tokio::test]
    async fn test_read_nonexistent_file() {
        let tool = ReadTool;
        let result = tool.execute(serde_json::json!({"path": "/tmp/nonexistent-file-12345"}))
            .await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Failed to read file"));
    }

    #[tokio::test]
    async fn test_read_success() {
        let tmp = std::env::temp_dir().join("aibsd_test_read.txt");
        std::fs::write(&tmp, "line1\nline2\nline3\n").unwrap();

        let tool = ReadTool;
        let result = tool.execute(serde_json::json!({"path": tmp.to_str().unwrap()})).await;
        assert!(result.success);
        assert_eq!(result.output, "1:line1\n2:line2\n3:line3");

        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn test_read_with_offset() {
        let tmp = std::env::temp_dir().join("aibsd_test_read_offset.txt");
        std::fs::write(&tmp, "a\nb\nc\nd\ne\n").unwrap();

        let tool = ReadTool;
        let result = tool.execute(serde_json::json!({"path": tmp.to_str().unwrap(), "offset": 3}))
            .await;
        assert!(result.success);
        assert_eq!(result.output, "3:c\n4:d\n5:e");

        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn test_read_with_limit() {
        let tmp = std::env::temp_dir().join("aibsd_test_read_limit.txt");
        std::fs::write(&tmp, "1\n2\n3\n4\n5\n").unwrap();

        let tool = ReadTool;
        let result = tool.execute(serde_json::json!({"path": tmp.to_str().unwrap(), "offset": 1, "limit": 2}))
            .await;
        assert!(result.success);
        assert_eq!(result.output, "1:1\n2:2");

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_read_input_schema() {
        let tool = ReadTool;
        let schema = tool.input_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["required"].as_array().unwrap().iter().any(|v| v == "path"));
    }
}
