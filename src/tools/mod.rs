pub mod registry;
pub mod core;
pub mod freebsd;

use std::fmt;

#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

impl ToolOutput {
    pub fn ok(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            error: None,
        }
    }

    pub fn err(error: impl Into<String>) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(error.into()),
        }
    }
}

impl fmt::Display for ToolOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.success {
            write!(f, "{}", self.output)
        } else {
            write!(f, "Error: {}", self.error.as_deref().unwrap_or("unknown"))
        }
    }
}

#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> serde_json::Value;
    async fn execute(&self, args: serde_json::Value) -> ToolOutput;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_output_ok() {
        let out = ToolOutput::ok("success");
        assert_eq!(out.success, true);
        assert_eq!(out.output, "success");
        assert_eq!(out.error, None);
    }

    #[test]
    fn test_tool_output_err() {
        let out = ToolOutput::err("something failed");
        assert_eq!(out.success, false);
        assert_eq!(out.output, "");
        assert_eq!(out.error, Some("something failed".to_string()));
    }

    #[test]
    fn test_tool_output_display_ok() {
        let out = ToolOutput::ok("hello");
        assert_eq!(out.to_string(), "hello");
    }

    #[test]
    fn test_tool_output_display_err() {
        let out = ToolOutput::err("fail");
        assert_eq!(out.to_string(), "Error: fail");
    }

    #[test]
    fn test_tool_output_ok_empty() {
        let out = ToolOutput::ok("");
        assert!(out.success);
        assert_eq!(out.output, "");
    }

    #[test]
    fn test_tool_output_into_string() {
        let out = ToolOutput::ok(String::from("test"));
        assert_eq!(out.output, "test");
    }

    struct TestTool;

    #[async_trait::async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &str { "test" }
        fn description(&self) -> &str { "a test tool" }
        fn input_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }
        async fn execute(&self, _args: serde_json::Value) -> ToolOutput {
            ToolOutput::ok("ran")
        }
    }

    #[tokio::test]
    async fn test_tool_trait() {
        let tool = TestTool;
        assert_eq!(tool.name(), "test");
        assert_eq!(tool.description(), "a test tool");
        let schema = tool.input_schema();
        assert_eq!(schema["type"], "object");
        let result = tool.execute(serde_json::json!({})).await;
        assert_eq!(result.output, "ran");
    }
}
