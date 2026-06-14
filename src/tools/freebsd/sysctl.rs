use crate::tools::{Tool, ToolOutput};
use serde_json::Value;
use tokio::process::Command;
use std::process::Stdio;

pub struct SysctlTool;

#[async_trait::async_trait]
impl Tool for SysctlTool {
    fn name(&self) -> &str { "freebsd_sysctl" }
    fn description(&self) -> &str {
        "Read and write FreeBSD kernel parameters via sysctl. Supports listing all parameters, reading specific ones, and setting values."
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["read", "write", "list", "search", "describe"],
                    "description": "Operation to perform"
                },
                "key": {
                    "type": "string",
                    "description": "sysctl OID (e.g. kern.hostname, net.inet.ip.forwarding)"
                },
                "value": {
                    "type": "string",
                    "description": "Value to set (for write action)"
                },
                "search": {
                    "type": "string",
                    "description": "Search pattern (for search action)"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: Value) -> ToolOutput {
        if !Self::is_freebsd() {
            return ToolOutput::err("this tool only works on FreeBSD");
        }

        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");

        match action {
            "list" => self.sysctl("-a").await,
            "read" => {
                let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("");
                if key.is_empty() { return ToolOutput::err("key is required"); }
                self.sysctl(&format!("-n {}", key)).await
            }
            "write" => {
                let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("");
                let value = args.get("value").and_then(|v| v.as_str()).unwrap_or("");
                if key.is_empty() { return ToolOutput::err("key is required"); }
                if value.is_empty() { return ToolOutput::err("value is required"); }
                self.sysctl_write(key, value).await
            }
            "search" => {
                let pattern = args.get("search").and_then(|v| v.as_str()).unwrap_or("");
                if pattern.is_empty() { return ToolOutput::err("search pattern is required"); }
                self.sysctl(&format!("-a | rg {}", pattern)).await
            }
            "describe" => {
                let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("");
                if key.is_empty() { return ToolOutput::err("key is required"); }
                self.sysctl(&format!("-d {}", key)).await
            }
            _ => ToolOutput::err(format!("Unknown action: {}", action)),
        }
    }
}

impl SysctlTool {
    fn is_freebsd() -> bool {
        cfg!(target_os = "freebsd")
    }

    async fn sysctl(&self, args: &str) -> ToolOutput {
        let shell = if cfg!(target_os = "freebsd") { "/bin/sh" } else { "/bin/bash" };
        let output = Command::new(shell)
            .arg("-c")
            .arg(format!("sysctl {}", args))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output().await;

        match output {
            Ok(o) => {
                let out = String::from_utf8_lossy(&o.stdout).to_string();
                let err = String::from_utf8_lossy(&o.stderr).to_string();
                if o.status.success() {
                    ToolOutput::ok(out.trim().to_string())
                } else {
                    ToolOutput::err(err.trim().to_string())
                }
            }
            Err(e) => ToolOutput::err(format!("sysctl failed: {}", e)),
        }
    }

    async fn sysctl_write(&self, key: &str, value: &str) -> ToolOutput {
        let output = Command::new("sysctl")
            .arg(format!("{}={}", key, value))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output().await;

        match output {
            Ok(o) => {
                let out = String::from_utf8_lossy(&o.stdout).to_string();
                let err = String::from_utf8_lossy(&o.stderr).to_string();
                if o.status.success() {
                    ToolOutput::ok(format!("{} = {}\n{}", key, value, out.trim()))
                } else {
                    ToolOutput::err(err.trim().to_string())
                }
            }
            Err(e) => ToolOutput::err(format!("sysctl failed: {}", e)),
        }
    }
}
