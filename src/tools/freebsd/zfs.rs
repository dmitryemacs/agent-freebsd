use crate::tools::{Tool, ToolOutput};
use serde_json::Value;
use tokio::process::Command;
use std::process::Stdio;

pub struct ZfsTool;

#[async_trait::async_trait]
impl Tool for ZfsTool {
    fn name(&self) -> &str { "freebsd_zfs" }
    fn description(&self) -> &str {
        "Manage ZFS datasets and snapshots on FreeBSD: list, create, destroy, snapshot, rollback, list snapshots."
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list", "create", "destroy", "snapshot", "rollback", "list_snapshots", "list_clones", "info"],
                    "description": "Operation to perform"
                },
                "dataset": {
                    "type": "string",
                    "description": "ZFS dataset name (e.g. zroot/usr/home)"
                },
                "snapshot_name": {
                    "type": "string",
                    "description": "Snapshot name (for snapshot/rollback actions)"
                },
                "mountpoint": {
                    "type": "string",
                    "description": "Mount point (for create action)"
                },
                "properties": {
                    "type": "string",
                    "description": "Additional properties as key=value pairs, comma-separated (for create)"
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
        let dataset = args.get("dataset").and_then(|v| v.as_str()).unwrap_or("");

        match action {
            "list" => self.list_datasets(dataset).await,
            "create" => self.create_dataset(&args).await,
            "destroy" => self.destroy_dataset(dataset).await,
            "snapshot" => {
                let snap = args.get("snapshot_name").and_then(|v| v.as_str()).unwrap_or("");
                self.create_snapshot(dataset, snap).await
            }
            "rollback" => {
                let snap = args.get("snapshot_name").and_then(|v| v.as_str()).unwrap_or("");
                self.rollback_snapshot(dataset, snap).await
            }
            "list_snapshots" => self.list_snapshots(dataset).await,
            "list_clones" => self.list_clones(dataset).await,
            "info" => self.zfs_info(dataset).await,
            _ => ToolOutput::err(format!("Unknown action: {}", action)),
        }
    }
}

impl ZfsTool {
    fn is_freebsd() -> bool {
        cfg!(target_os = "freebsd")
    }

    fn zfs(args: &[&str]) -> Command {
        let mut cmd = Command::new("zfs");
        cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
        cmd
    }

    async fn list_datasets(&self, dataset: &str) -> ToolOutput {
        let mut args = vec!["list", "-r", "-t", "filesystem,volume"];
        if !dataset.is_empty() {
            args.push(dataset);
        }
        let output = Self::zfs(&args).output().await;
        Self::output_result(output)
    }

    async fn create_dataset(&self, args: &Value) -> ToolOutput {
        let dataset = args.get("dataset").and_then(|v| v.as_str()).unwrap_or("");
        if dataset.is_empty() {
            return ToolOutput::err("dataset is required");
        }

        let mountpoint = args.get("mountpoint").and_then(|v| v.as_str());
        let properties = args.get("properties").and_then(|v| v.as_str());

        let mut cmd_args: Vec<String> = vec!["create".to_string()];
        if let Some(mp) = mountpoint {
            cmd_args.push("-o".to_string());
            cmd_args.push(format!("mountpoint={}", mp));
        }
        if let Some(props) = properties {
            for prop in props.split(',') {
                cmd_args.push("-o".to_string());
                cmd_args.push(prop.trim().to_string());
            }
        }
        cmd_args.push(dataset.to_string());

        let refs: Vec<&str> = cmd_args.iter().map(|s| s.as_str()).collect();
        let output = Self::zfs(&refs).output().await;
        Self::output_result(output)
    }

    async fn destroy_dataset(&self, dataset: &str) -> ToolOutput {
        if dataset.is_empty() {
            return ToolOutput::err("dataset is required");
        }
        let output = Self::zfs(&["destroy", "-r", dataset]).output().await;
        Self::output_result(output)
    }

    async fn create_snapshot(&self, dataset: &str, snapshot: &str) -> ToolOutput {
        if dataset.is_empty() || snapshot.is_empty() {
            return ToolOutput::err("dataset and snapshot_name are required");
        }
        let name = format!("{}@{}", dataset, snapshot);
        let output = Self::zfs(&["snapshot", &name]).output().await;
        Self::output_result(output)
    }

    async fn rollback_snapshot(&self, dataset: &str, snapshot: &str) -> ToolOutput {
        if dataset.is_empty() || snapshot.is_empty() {
            return ToolOutput::err("dataset and snapshot_name are required");
        }
        let name = format!("{}@{}", dataset, snapshot);
        let output = Self::zfs(&["rollback", &name]).output().await;
        Self::output_result(output)
    }

    async fn list_snapshots(&self, dataset: &str) -> ToolOutput {
        let mut args = vec!["list", "-r", "-t", "snapshot"];
        if !dataset.is_empty() {
            args.push(dataset);
        }
        let output = Self::zfs(&args).output().await;
        Self::output_result(output)
    }

    async fn list_clones(&self, dataset: &str) -> ToolOutput {
        let mut args = vec!["list", "-r", "-t", "filesystem"];
        if !dataset.is_empty() {
            args.push(dataset);
        }
        let output = Self::zfs(&args).output().await;
        Self::output_result(output)
    }

    async fn zfs_info(&self, dataset: &str) -> ToolOutput {
        if dataset.is_empty() {
            return ToolOutput::err("dataset is required");
        }
        let output = Self::zfs(&["get", "all", dataset]).output().await;
        Self::output_result(output)
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
            Err(e) => ToolOutput::err(format!("zfs failed: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Tool;

    #[tokio::test]
    async fn test_non_freebsd_guard() {
        let tool = ZfsTool;
        let result = tool.execute(serde_json::json!({"action": "list"})).await;
        assert!(!result.success);
        assert_eq!(result.error.unwrap(), "this tool only works on FreeBSD");
    }

    #[tokio::test]
    async fn test_zfs_unknown_action() {
        let tool = ZfsTool;
        let result = tool.execute(serde_json::json!({"action": "bogus"})).await;
        assert!(!result.success);
        if cfg!(target_os = "freebsd") {
            assert!(result.error.unwrap().contains("Unknown action"));
        }
    }

    #[test]
    fn test_zfs_input_schema() {
        let tool = ZfsTool;
        let schema = tool.input_schema();
        assert!(schema["required"].as_array().unwrap().iter().any(|v| v == "action"));
    }
}
