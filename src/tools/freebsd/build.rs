use crate::tools::{Tool, ToolOutput};
use serde_json::Value;
use tokio::process::Command;
use std::process::Stdio;

pub struct BuildTool;

#[async_trait::async_trait]
impl Tool for BuildTool {
    fn name(&self) -> &str { "freebsd_build" }
    fn description(&self) -> &str {
        "Build FreeBSD base system from source: buildworld, buildkernel, installworld, installkernel, show build options, clean."
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["buildworld", "buildkernel", "installworld", "installkernel", "cleanworld", "cleankernel", "clean", "show_config"],
                    "description": "Operation to perform"
                },
                "src_dir": {
                    "type": "string",
                    "description": "Source directory (default: /usr/src)"
                },
                "kernel_conf": {
                    "type": "string",
                    "description": "Kernel configuration file (for buildkernel, default: GENERIC)"
                },
                "jobs": {
                    "type": "integer",
                    "description": "Number of parallel make jobs (default: number of CPUs)",
                    "default": 0
                },
                "options": {
                    "type": "string",
                    "description": "Additional make options (e.g. WITHOUT_CLANG=yes WITH_DEBUG=yes)"
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
        let src_dir = args.get("src_dir").and_then(|v| v.as_str()).unwrap_or("/usr/src");
        let jobs = args.get("jobs").and_then(|v| v.as_i64()).unwrap_or(0);
        let options = args.get("options").and_then(|v| v.as_str()).unwrap_or("");

        match action {
            "buildworld" => self.make(src_dir, "buildworld", jobs, options).await,
            "buildkernel" => {
                let kernel = args.get("kernel_conf").and_then(|v| v.as_str()).unwrap_or("GENERIC");
                self.make(src_dir, &format!("buildkernel KERNCONF={}", kernel), jobs, options).await
            }
            "installworld" => self.make(src_dir, "installworld", jobs, options).await,
            "installkernel" => {
                let kernel = args.get("kernel_conf").and_then(|v| v.as_str()).unwrap_or("GENERIC");
                self.make(src_dir, &format!("installkernel KERNCONF={}", kernel), jobs, options).await
            }
            "cleanworld" => self.make(src_dir, "cleanworld", jobs, options).await,
            "cleankernel" => self.make(src_dir, "cleankernel", jobs, options).await,
            "clean" => self.make(src_dir, "clean", jobs, options).await,
            "show_config" => self.show_config(src_dir).await,
            _ => ToolOutput::err(format!("Unknown action: {}", action)),
        }
    }
}

impl BuildTool {
    fn is_freebsd() -> bool {
        cfg!(target_os = "freebsd")
    }

    fn make_cmd(src_dir: &str, target: &str, jobs: i64, options: &str) -> Command {
        let mut cmd = Command::new("make");
        cmd.current_dir(src_dir);

        if jobs > 0 {
            cmd.arg("-j").arg(jobs.to_string());
        }

        cmd.arg(target);

        if !options.is_empty() {
            for opt in options.split_whitespace() {
                cmd.arg(opt);
            }
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        cmd
    }

    async fn make(&self, src_dir: &str, target: &str, jobs: i64, options: &str) -> ToolOutput {
        let mut cmd = Self::make_cmd(src_dir, target, jobs, options);
        let output = cmd.output().await;

        match output {
            Ok(o) => {
                let out = String::from_utf8_lossy(&o.stdout).to_string();
                let err = String::from_utf8_lossy(&o.stderr).to_string();
                if o.status.success() {
                    ToolOutput::ok(format!("{} completed\n{}", target, out.trim()))
                } else {
                    ToolOutput::err(format!("{} failed (exit {}):\n{}", target,
                        o.status.code().unwrap_or(-1), err.trim()))
                }
            }
            Err(e) => ToolOutput::err(format!("make failed: {}", e)),
        }
    }

    async fn show_config(&self, src_dir: &str) -> ToolOutput {
        let output = Self::make_cmd(src_dir, "showconfig", 0, "")
            .output().await;

        match output {
            Ok(o) => {
                let out = String::from_utf8_lossy(&o.stdout).to_string();
                if o.status.success() {
                    ToolOutput::ok(out.trim().to_string())
                } else {
                    ToolOutput::err("Failed to show build config")
                }
            }
            Err(e) => ToolOutput::err(format!("make showconfig failed: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_non_freebsd_guard() {
        let tool = BuildTool;
        let result = tool.execute(serde_json::json!({"action": "buildworld"})).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("only works on FreeBSD"));
    }

    #[tokio::test]
    async fn test_build_unknown_action() {
        let tool = BuildTool;
        let result = tool.execute(serde_json::json!({"action": "bogus"})).await;
        assert!(!result.success);
        if cfg!(target_os = "freebsd") {
            assert!(result.error.unwrap().contains("Unknown action"));
        }
    }

    #[test]
    fn test_build_input_schema() {
        let tool = BuildTool;
        let schema = tool.input_schema();
        assert!(schema["required"].as_array().unwrap().iter().any(|v| v == "action"));
    }
}
