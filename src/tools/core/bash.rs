use crate::tools::{Tool, ToolOutput};
use serde_json::Value;
use std::process::Stdio;

pub struct BashTool;

#[async_trait::async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str { "bash" }
    fn description(&self) -> &str { "Execute a bash command on the system. Use for running commands, scripts, and interacting with the system." }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The bash command to execute"
                },
                "workdir": {
                    "type": "string",
                    "description": "Working directory for the command (optional)"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in milliseconds (optional, default 30000)",
                    "default": 30000
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: Value) -> ToolOutput {
        let command = args.get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if command.is_empty() {
            return ToolOutput::err("command is required");
        }

        let workdir = args.get("workdir")
            .and_then(|v| v.as_str());

        let _timeout_ms = args.get("timeout")
            .and_then(|v| v.as_i64())
            .unwrap_or(30000) as u64;

        let shell = if cfg!(target_os = "freebsd") { "/bin/sh" } else { "/bin/bash" };

        let mut cmd = tokio::process::Command::new(shell);
        cmd.arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(dir) = workdir {
            cmd.current_dir(dir);
        }

        match cmd.output().await {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                let result = if stdout.is_empty() && !stderr.is_empty() {
                    stderr
                } else {
                    if !stderr.is_empty() {
                        format!("{}\n{}", stdout, stderr)
                    } else {
                        stdout
                    }
                };

                if output.status.success() {
                    ToolOutput::ok(result.trim().to_string())
                } else {
                    ToolOutput::err(format!("Exit code {}: {}", output.status.code().unwrap_or(-1), result.trim()))
                }
            }
            Err(e) => ToolOutput::err(format!("Failed to execute command: {}", e)),
        }
    }
}
