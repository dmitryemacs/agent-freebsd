use crate::tools::{Tool, ToolOutput};
use serde_json::Value;
use tokio::process::Command;
use std::process::Stdio;

pub struct ServiceTool;

#[async_trait::async_trait]
impl Tool for ServiceTool {
    fn name(&self) -> &str { "freebsd_service" }
    fn description(&self) -> &str {
        "Manage FreeBSD services: start, stop, restart, status, enable, disable, list, describe."
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["start", "stop", "restart", "status", "enable", "disable", "list", "describe"],
                    "description": "Operation to perform"
                },
                "service": {
                    "type": "string",
                    "description": "Service name (e.g. nginx, sshd, jail)"
                },
                "extra_args": {
                    "type": "string",
                    "description": "Extra arguments passed to the service command"
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
            "list" => self.list_services().await,
            "describe" => {
                let svc = args.get("service").and_then(|v| v.as_str()).unwrap_or("");
                self.describe_service(svc).await
            }
            _ => {
                let svc = args.get("service").and_then(|v| v.as_str()).unwrap_or("");
                let extra = args.get("extra_args").and_then(|v| v.as_str()).unwrap_or("");
                self.service_action(action, svc, extra).await
            }
        }
    }
}

impl ServiceTool {
    fn is_freebsd() -> bool {
        cfg!(target_os = "freebsd")
    }

    fn service(args: &[&str]) -> Command {
        let mut cmd = Command::new("service");
        cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
        cmd
    }

    async fn list_services(&self) -> ToolOutput {
        let output = Self::service(&["-e"]).output().await;
        Self::output_result(output)
    }

    async fn describe_service(&self, name: &str) -> ToolOutput {
        if name.is_empty() {
            return ToolOutput::err("service name is required");
        }
        let output = Self::service(&[name, "describe"]).output().await;
        Self::output_result(output)
    }

    async fn service_action(&self, action: &str, service: &str, extra: &str) -> ToolOutput {
        if service.is_empty() {
            return ToolOutput::err("service name is required");
        }

        let mut args = vec![service, action];
        if !extra.is_empty() {
            args.push(extra);
        }

        let output = Self::service(&args).output().await;
        match &output {
            Ok(o) => {
                let out = String::from_utf8_lossy(&o.stdout).to_string();
                let err = String::from_utf8_lossy(&o.stderr).to_string();
                if o.status.success() {
                    ToolOutput::ok(format!("{} {}: {}", action, service, out.trim()))
                } else {
                    ToolOutput::err(format!("{} {} failed: {}", action, service, err.trim()))
                }
            }
            Err(e) => ToolOutput::err(format!("service failed: {}", e)),
        }
    }

    fn output_result(result: Result<std::process::Output, std::io::Error>) -> ToolOutput {
        match result {
            Ok(o) => {
                let out = String::from_utf8_lossy(&o.stdout).to_string();
                let err = String::from_utf8_lossy(&o.stderr).to_string();
                if o.status.success() {
                    ToolOutput::ok(out.trim().to_string())
                } else {
                    ToolOutput::err(err.trim().to_string())
                }
            }
            Err(e) => ToolOutput::err(format!("service failed: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_non_freebsd_guard() {
        let tool = ServiceTool;
        let result = tool.execute(serde_json::json!({"action": "list"})).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("only works on FreeBSD"));
    }

    #[tokio::test]
    async fn test_service_unknown_action() {
        let tool = ServiceTool;
        let result = tool.execute(serde_json::json!({"action": "bogus"})).await;
        assert!(!result.success);
        if cfg!(target_os = "freebsd") {
            assert!(result.error.unwrap().contains("Unknown action"));
        }
    }

    #[test]
    fn test_service_input_schema() {
        let tool = ServiceTool;
        let schema = tool.input_schema();
        assert!(schema["required"].as_array().unwrap().iter().any(|v| v == "action"));
    }
}
