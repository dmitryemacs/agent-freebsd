use crate::tools::{Tool, ToolOutput};
use serde_json::Value;
use tokio::process::Command;
use std::process::Stdio;

pub struct PkgTool;

#[async_trait::async_trait]
impl Tool for PkgTool {
    fn name(&self) -> &str { "freebsd_pkg" }
    fn description(&self) -> &str {
        "Manage FreeBSD packages with pkg: search, install, remove, update, upgrade, audit, info, autoremove."
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["search", "install", "remove", "update", "upgrade", "audit", "info", "autoremove", "which"],
                    "description": "Operation to perform"
                },
                "packages": {
                    "type": "string",
                    "description": "Package name(s), space-separated for multiple"
                },
                "yes": {
                    "type": "boolean",
                    "description": "Assume yes to prompts (default: true)",
                    "default": true
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
        let packages = args.get("packages").and_then(|v| v.as_str()).unwrap_or("");
        let yes = args.get("yes").and_then(|v| v.as_bool()).unwrap_or(true);

        match action {
            "search" => self.search(packages).await,
            "install" => self.install(packages, yes).await,
            "remove" => self.remove(packages, yes).await,
            "update" => self.update().await,
            "upgrade" => self.upgrade(yes).await,
            "audit" => self.audit().await,
            "info" => self.info(packages).await,
            "autoremove" => self.autoremove(yes).await,
            "which" => self.which(packages).await,
            _ => ToolOutput::err(format!("Unknown action: {}", action)),
        }
    }
}

impl PkgTool {
    fn is_freebsd() -> bool {
        cfg!(target_os = "freebsd")
    }

    fn pkg(args: &[&str]) -> Command {
        let mut cmd = Command::new("pkg");
        cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
        cmd
    }

    async fn search(&self, query: &str) -> ToolOutput {
        if query.is_empty() { return ToolOutput::err("search query is required"); }
        let output = Self::pkg(&["search", query]).output().await;
        Self::output_result(output)
    }

    async fn install(&self, packages: &str, yes: bool) -> ToolOutput {
        if packages.is_empty() { return ToolOutput::err("packages is required"); }
        let mut args = vec!["install"];
        if yes { args.push("-y"); }
        for pkg in packages.split_whitespace() {
            args.push(pkg);
        }
        let output = Self::pkg(&args).output().await;
        Self::output_result(output)
    }

    async fn remove(&self, packages: &str, yes: bool) -> ToolOutput {
        if packages.is_empty() { return ToolOutput::err("packages is required"); }
        let mut args = vec!["remove"];
        if yes { args.push("-y"); }
        for pkg in packages.split_whitespace() {
            args.push(pkg);
        }
        let output = Self::pkg(&args).output().await;
        Self::output_result(output)
    }

    async fn update(&self) -> ToolOutput {
        let output = Self::pkg(&["update"]).output().await;
        Self::output_result(output)
    }

    async fn upgrade(&self, yes: bool) -> ToolOutput {
        let mut args = vec!["upgrade"];
        if yes { args.push("-y"); }
        let output = Self::pkg(&args).output().await;
        Self::output_result(output)
    }

    async fn audit(&self) -> ToolOutput {
        let output = Self::pkg(&["audit", "-F"]).output().await;
        Self::output_result(output)
    }

    async fn info(&self, packages: &str) -> ToolOutput {
        let mut args = vec!["info"];
        if !packages.is_empty() {
            args.push(packages);
        }
        let output = Self::pkg(&args).output().await;
        Self::output_result(output)
    }

    async fn autoremove(&self, yes: bool) -> ToolOutput {
        let mut args = vec!["autoremove"];
        if yes { args.push("-y"); }
        let output = Self::pkg(&args).output().await;
        Self::output_result(output)
    }

    async fn which(&self, file: &str) -> ToolOutput {
        if file.is_empty() { return ToolOutput::err("file path is required"); }
        let output = Self::pkg(&["which", file]).output().await;
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
            Err(e) => ToolOutput::err(format!("pkg failed: {}", e)),
        }
    }
}
