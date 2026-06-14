use crate::tools::{Tool, ToolOutput};
use serde_json::Value;
use tokio::process::Command;
use std::process::Stdio;

pub struct GrepTool;

#[async_trait::async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str { "grep" }
    fn description(&self) -> &str { "Search file contents using a regular expression. Returns matching file paths and line numbers." }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regular expression to search for"
                },
                "include": {
                    "type": "string",
                    "description": "File glob pattern to filter (e.g. *.rs, *.{ts,tsx})"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search in (optional)"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: Value) -> ToolOutput {
        let pattern = args.get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if pattern.is_empty() {
            return ToolOutput::err("pattern is required");
        }

        let path = args.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let mut rg = Command::new("rg");
        rg.arg("-n")
            .arg("--color")
            .arg("never")
            .arg(pattern)
            .arg(path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(include) = args.get("include").and_then(|v| v.as_str()) {
            if !include.is_empty() {
                rg.arg("-g").arg(include);
            }
        }

        match rg.output().await {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                if !stderr.is_empty() && stdout.is_empty() {
                    ToolOutput::err(stderr.trim().to_string())
                } else if stdout.trim().is_empty() {
                    ToolOutput::ok("No matches found")
                } else {
                    ToolOutput::ok(stdout.trim().to_string())
                }
            }
            Err(e) => ToolOutput::err(format!("Failed to search: {}", e)),
        }
    }
}
