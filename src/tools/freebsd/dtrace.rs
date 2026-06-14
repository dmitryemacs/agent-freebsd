use crate::tools::{Tool, ToolOutput};
use serde_json::Value;
use tokio::process::Command;
use std::process::Stdio;

pub struct DTraceTool;

#[async_trait::async_trait]
impl Tool for DTraceTool {
    fn name(&self) -> &str { "freebsd_dtrace" }
    fn description(&self) -> &str {
        "Run DTrace one-liners and scripts on FreeBSD for system tracing. Supports predefined probes and custom D scripts."
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["syscalls", "opens", "execs", "io", "net", "profile", "run"],
                    "description": "Predefined probe or 'run' for custom script"
                },
                "command": {
                    "type": "string",
                    "description": "Custom DTrace script or one-liner (for 'run' action)"
                },
                "duration": {
                    "type": "integer",
                    "description": "Sampling duration in seconds (default: 5, max: 30)",
                    "default": 5
                },
                "pid": {
                    "type": "integer",
                    "description": "Process ID to trace (optional)"
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
        let duration = args.get("duration").and_then(|v| v.as_i64()).unwrap_or(5);
        let pid = args.get("pid").and_then(|v| v.as_i64());

        let script = match action {
            "syscalls" => {
                let _ = pid;
                format!("syscall:::entry {{ @[probefunc] = count(); }} tick-{}s {{ exit(0); }}", duration)
            }
            "opens" => {
                format!("syscall::open*:entry {{ printf(\"%s %s\\n\", execname, copyinstr(arg0)); }} tick-{}s {{ exit(0); }}", duration)
            }
            "execs" => {
                format!("proc:exec:success {{ printf(\"%s\\n\", execname); }} tick-{}s {{ exit(0); }}", duration)
            }
            "io" => {
                format!("io:::start {{ @[execname] = sum(args[0]->b_bufsize); }} tick-{}s {{ exit(0); }}", duration)
            }
            "net" => {
                format!("ip:::send {{ @[args[2]->ip_saddr, args[3]->ip_daddr] = count(); }} tick-{}s {{ exit(0); }}", duration)
            }
            "profile" => {
                format!("profile:::profile-99 {{ @[execname] = count(); }} tick-{}s {{ exit(0); }}", duration)
            }
            "run" => {
                let custom = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
                if custom.is_empty() {
                    return ToolOutput::err("command is required for 'run' action");
                }
                custom.to_string()
            }
            _ => return ToolOutput::err(format!("Unknown action: {}", action)),
        };

        let mut cmd = Command::new("dtrace");
        cmd.arg("-q").arg("-n").arg(&script)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(p) = pid {
            cmd.arg("-p").arg(p.to_string());
        }

        let output = cmd.output().await;
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
            Err(e) => ToolOutput::err(format!("dtrace failed: {}", e)),
        }
    }
}

impl DTraceTool {
    fn is_freebsd() -> bool {
        cfg!(target_os = "freebsd")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_non_freebsd_guard() {
        let tool = DTraceTool;
        let result = tool.execute(serde_json::json!({"action": "syscalls"})).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("only works on FreeBSD"));
    }

    #[tokio::test]
    async fn test_dtrace_run_missing_command() {
        let tool = DTraceTool;
        let result = tool.execute(serde_json::json!({"action": "run"})).await;
        assert!(!result.success);
        if cfg!(target_os = "freebsd") {
            assert_eq!(result.error.unwrap(), "command is required for 'run' action");
        }
    }

    #[tokio::test]
    async fn test_dtrace_unknown_action() {
        let tool = DTraceTool;
        let result = tool.execute(serde_json::json!({"action": "bogus"})).await;
        assert!(!result.success);
        if cfg!(target_os = "freebsd") {
            assert!(result.error.unwrap().contains("Unknown action"));
        }
    }

    #[test]
    fn test_dtrace_input_schema() {
        let tool = DTraceTool;
        let schema = tool.input_schema();
        assert!(schema["required"].as_array().unwrap().iter().any(|v| v == "action"));
    }
}
