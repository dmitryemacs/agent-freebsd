use crate::tools::{Tool, ToolOutput};
use serde_json::Value;
use tokio::process::Command;
use std::process::Stdio;

pub struct PfTool;

#[async_trait::async_trait]
impl Tool for PfTool {
    fn name(&self) -> &str { "freebsd_pf" }
    fn description(&self) -> &str {
        "Manage FreeBSD PF (Packet Filter) firewall: show rules, stats, states, NAT, enable/disable, load config."
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["show_rules", "show_stats", "show_states", "show_nat", "enable", "disable", "load", "flush"],
                    "description": "Operation to perform"
                },
                "config_file": {
                    "type": "string",
                    "description": "Path to pf.conf (for load action)"
                },
                "table": {
                    "type": "string",
                    "description": "PF table name (for table operations)"
                },
                "table_action": {
                    "type": "string",
                    "enum": ["add", "delete", "show"],
                    "description": "Table operation"
                },
                "address": {
                    "type": "string",
                    "description": "IP address for table operations"
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
            "show_rules" => Self::simple_cmd(&["-s", "rules"]).await,
            "show_stats" => Self::simple_cmd(&["-s", "info"]).await,
            "show_states" => Self::simple_cmd(&["-s", "states"]).await,
            "show_nat" => Self::simple_cmd(&["-s", "nat"]).await,
            "enable" => Self::simple_cmd(&["-e"]).await,
            "disable" => Self::simple_cmd(&["-d"]).await,
            "load" => {
                let config = args.get("config_file").and_then(|v| v.as_str()).unwrap_or("");
                if config.is_empty() {
                    return ToolOutput::err("config_file is required for load");
                }
                Self::simple_cmd(&["-f", config]).await
            }
            "flush" => Self::simple_cmd(&["-F", "all"]).await,
            _ => {
                let table = args.get("table").and_then(|v| v.as_str()).unwrap_or("");
                let table_action = args.get("table_action").and_then(|v| v.as_str()).unwrap_or("");
                let address = args.get("address").and_then(|v| v.as_str()).unwrap_or("");
                self.table_op(table, table_action, address).await
            }
        }
    }
}

impl PfTool {
    fn is_freebsd() -> bool {
        cfg!(target_os = "freebsd")
    }

    fn pfctl(args: &[&str]) -> Command {
        let mut cmd = Command::new("pfctl");
        cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
        cmd
    }

    async fn simple_cmd(args: &[&str]) -> ToolOutput {
        let output = Self::pfctl(args).output().await;
        Self::output_result(output)
    }

    async fn table_op(&self, table: &str, action: &str, address: &str) -> ToolOutput {
        if table.is_empty() {
            return ToolOutput::err("table is required for table operations");
        }

        match action {
            "show" => Self::simple_cmd(&["-t", table, "-T", "show"]).await,
            "add" => {
                if address.is_empty() {
                    return ToolOutput::err("address is required for add");
                }
                Self::simple_cmd(&["-t", table, "-T", "add", address]).await
            }
            "delete" => {
                if address.is_empty() {
                    return ToolOutput::err("address is required for delete");
                }
                Self::simple_cmd(&["-t", table, "-T", "delete", address]).await
            }
            _ => ToolOutput::err(format!("Unknown table action: {}", action)),
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
            Err(e) => ToolOutput::err(format!("pfctl failed: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_non_freebsd_guard() {
        let tool = PfTool;
        let result = tool.execute(serde_json::json!({"action": "list"})).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("only works on FreeBSD"));
    }

    #[tokio::test]
    async fn test_pf_unknown_action() {
        let tool = PfTool;
        let result = tool.execute(serde_json::json!({"action": "bogus"})).await;
        assert!(!result.success);
        if cfg!(target_os = "freebsd") {
            assert!(result.error.unwrap().contains("Unknown action"));
        }
    }

    #[test]
    fn test_pf_input_schema() {
        let tool = PfTool;
        let schema = tool.input_schema();
        assert!(schema["required"].as_array().unwrap().iter().any(|v| v == "action"));
    }
}
