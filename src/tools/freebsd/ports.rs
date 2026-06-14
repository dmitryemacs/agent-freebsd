use crate::tools::{Tool, ToolOutput};
use serde_json::Value;
use tokio::process::Command;
use std::process::Stdio;

pub struct PortsTool;

#[async_trait::async_trait]
impl Tool for PortsTool {
    fn name(&self) -> &str { "freebsd_ports" }
    fn description(&self) -> &str {
        "Manage FreeBSD ports tree: search, install, clean, fetch, list installed from ports."
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["search", "install", "fetch", "clean", "list", "whereis"],
                    "description": "Operation to perform"
                },
                "port": {
                    "type": "string",
                    "description": "Port name or path (e.g. www/nginx, /usr/ports/www/nginx)"
                },
                "query": {
                    "type": "string",
                    "description": "Search query (for search action)"
                },
                "ports_dir": {
                    "type": "string",
                    "description": "Ports directory (default: /usr/ports)"
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
            "search" => {
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                self.search(query).await
            }
            "install" => {
                let port = args.get("port").and_then(|v| v.as_str()).unwrap_or("");
                self.install(port).await
            }
            "fetch" => {
                let port = args.get("port").and_then(|v| v.as_str()).unwrap_or("");
                self.fetch(port).await
            }
            "clean" => {
                let port = args.get("port").and_then(|v| v.as_str()).unwrap_or("");
                self.clean(port).await
            }
            "list" => self.list_all().await,
            "whereis" => {
                let port = args.get("port").and_then(|v| v.as_str()).unwrap_or("");
                self.whereis(port).await
            }
            _ => ToolOutput::err(format!("Unknown action: {}", action)),
        }
    }
}

impl PortsTool {
    fn is_freebsd() -> bool {
        cfg!(target_os = "freebsd")
    }

    fn make(port_dir: &str, args: &[&str]) -> Command {
        let mut cmd = Command::new("make");
        cmd.args(args)
            .current_dir(port_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        cmd
    }

    async fn search(&self, query: &str) -> ToolOutput {
        if query.is_empty() {
            return ToolOutput::err("query is required");
        }
        let output = Self::make("/usr/ports", &["search", &format!("name={}", query)])
            .output().await;
        Self::output_result(output)
    }

    async fn install(&self, port: &str) -> ToolOutput {
        if port.is_empty() {
            return ToolOutput::err("port is required");
        }
        let port_dir = format!("/usr/ports/{}", port);
        let output = Self::make(&port_dir, &["install", "BATCH=yes"])
            .output().await;
        Self::output_result(output)
    }

    async fn fetch(&self, port: &str) -> ToolOutput {
        if port.is_empty() {
            return ToolOutput::err("port is required");
        }
        let port_dir = format!("/usr/ports/{}", port);
        let output = Self::make(&port_dir, &["fetch"]).output().await;
        Self::output_result(output)
    }

    async fn clean(&self, port: &str) -> ToolOutput {
        if port.is_empty() {
            return ToolOutput::err("port is required");
        }
        let port_dir = format!("/usr/ports/{}", port);
        let output = Self::make(&port_dir, &["clean"]).output().await;
        Self::output_result(output)
    }

    async fn list_all(&self) -> ToolOutput {
        let output = Command::new("pkg")
            .args(["info", "-q"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output().await;
        Self::output_result(output)
    }

    async fn whereis(&self, port: &str) -> ToolOutput {
        if port.is_empty() {
            return ToolOutput::err("port is required");
        }
        let output = Command::new("whereis")
            .arg(&format!("{}/", port))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output().await;
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
            Err(e) => ToolOutput::err(format!("ports failed: {}", e)),
        }
    }
}
