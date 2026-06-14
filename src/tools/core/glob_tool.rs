use crate::tools::{Tool, ToolOutput};
use serde_json::Value;

pub struct GlobTool;

#[async_trait::async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str { "glob" }
    fn description(&self) -> &str { "Search for files matching a glob pattern. Returns list of matching file paths." }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern (e.g. **/*.rs, src/**/*.ts)"
                },
                "path": {
                    "type": "string",
                    "description": "Base directory (optional, defaults to current)"
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

        let base_path = args.get("path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let base = base_path.as_deref()
            .or_else(|| Some("."))
            .unwrap();

        match glob::glob(&format!("{}/{}", base, pattern)) {
            Ok(entries) => {
                let paths: Vec<String> = entries
                    .filter_map(|e| e.ok())
                    .filter(|p| p.is_file())
                    .map(|p| p.to_string_lossy().to_string())
                    .collect();

                if paths.is_empty() {
                    ToolOutput::ok("No files matched the pattern")
                } else {
                    ToolOutput::ok(paths.join("\n"))
                }
            }
            Err(e) => ToolOutput::err(format!("Invalid glob pattern: {}", e)),
        }
    }
}
