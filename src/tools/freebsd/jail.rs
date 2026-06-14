use crate::tools::{Tool, ToolOutput};
use serde_json::Value;
use tokio::process::Command;
use std::process::Stdio;

pub struct JailTool;

#[async_trait::async_trait]
impl Tool for JailTool {
    fn name(&self) -> &str { "freebsd_jail" }
    fn description(&self) -> &str {
        "Manage FreeBSD jails: list, create, start, stop, exec commands inside jails."
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list", "create", "start", "stop", "restart", "exec", "info"],
                    "description": "Operation to perform"
                },
                "name": {
                    "type": "string",
                    "description": "Jail name (required for create/start/stop/exec)"
                },
                "hostname": {
                    "type": "string",
                    "description": "Hostname for the jail (create only)"
                },
                "ip": {
                    "type": "string",
                    "description": "IP address (create only, e.g. 192.168.1.100)"
                },
                "interface": {
                    "type": "string",
                    "description": "Network interface (create only, default: lo1)"
                },
                "path": {
                    "type": "string",
                    "description": "Jail root path (create only, default: /usr/local/jails/<name>)"
                },
                "command": {
                    "type": "string",
                    "description": "Command to execute inside jail (exec only)"
                },
                "user": {
                    "type": "string",
                    "description": "User to run command as (exec only, default: root)"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: Value) -> ToolOutput {
        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");
        let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");

        match action {
            "list" => self.list_jails().await,
            "create" => {
                if name.is_empty() {
                    return ToolOutput::err("name is required for create");
                }
                self.create_jail(&args).await
            }
            "start" => self.jail_action("start", name).await,
            "stop" => self.jail_action("stop", name).await,
            "restart" => {
                self.jail_action("stop", name).await;
                self.jail_action("start", name).await
            }
            "exec" => {
                let cmd = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
                let user = args.get("user").and_then(|v| v.as_str()).unwrap_or("root");
                self.exec_in_jail(name, user, cmd).await
            }
            "info" => self.jail_info(name).await,
            _ => ToolOutput::err(format!("Unknown action: {}", action)),
        }
    }
}

impl JailTool {
    fn is_freebsd() -> bool {
        cfg!(target_os = "freebsd")
    }

    fn not_freebsd() -> ToolOutput {
        ToolOutput::err("this tool only works on FreeBSD")
    }

    fn run_cmd(program: &str, args: &[&str]) -> Command {
        let mut cmd = Command::new(program);
        cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
        cmd
    }

    async fn list_jails(&self) -> ToolOutput {
        if !Self::is_freebsd() { return Self::not_freebsd(); }

        let output = Self::run_cmd("jls", &[]).output().await;
        match output {
            Ok(o) => {
                let out = String::from_utf8_lossy(&o.stdout).to_string();
                let err = String::from_utf8_lossy(&o.stderr).to_string();
                if o.status.success() {
                    if out.trim().is_empty() {
                        ToolOutput::ok("No jails running")
                    } else {
                        ToolOutput::ok(out.trim().to_string())
                    }
                } else {
                    ToolOutput::err(err.trim().to_string())
                }
            }
            Err(e) => ToolOutput::err(format!("jls failed: {}", e)),
        }
    }

    async fn create_jail(&self, args: &Value) -> ToolOutput {
        if !Self::is_freebsd() { return Self::not_freebsd(); }

        let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let hostname = args.get("hostname").and_then(|v| v.as_str()).unwrap_or(name);
        let ip = args.get("ip").and_then(|v| v.as_str()).unwrap_or("127.0.0.2");
        let interface = args.get("interface").and_then(|v| v.as_str()).unwrap_or("lo1");
        let default_path = format!("/usr/local/jails/{}", name);
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(&default_path);

        let config = format!(
            "{}{{\n  host.hostname = \"{}\";\n  ip4.addr = \"{}|{}\";\n  path = \"{}\";\n  mount.devfs;\n  exec.start = \"/bin/sh /etc/rc\";\n  exec.stop = \"/bin/sh /etc/rc.shutdown\";\n  persist;\n}}",
            name, hostname, interface, ip, path
        );

        let output = Self::run_cmd("jail", &["-c", "-f", "/dev/stdin"])
            .arg("-c")
            .arg(&config)
            .output().await;

        match output {
            Ok(o) => {
                let out = String::from_utf8_lossy(&o.stdout).to_string();
                let err = String::from_utf8_lossy(&o.stderr).to_string();
                if o.status.success() {
                    ToolOutput::ok(format!("Jail '{}' created and started\n{}", name, out.trim()))
                } else {
                    ToolOutput::err(err.trim().to_string())
                }
            }
            Err(e) => ToolOutput::err(format!("jail create failed: {}", e)),
        }
    }

    async fn jail_action(&self, action: &str, name: &str) -> ToolOutput {
        if !Self::is_freebsd() { return Self::not_freebsd(); }
        if name.is_empty() { return ToolOutput::err("name is required"); }

        let output = Self::run_cmd("service", &["jail", action, name]).output().await;
        match output {
            Ok(o) => {
                let out = String::from_utf8_lossy(&o.stdout).to_string();
                let err = String::from_utf8_lossy(&o.stderr).to_string();
                if o.status.success() {
                    ToolOutput::ok(format!("Jail '{}' {}ed\n{}", name, action, out.trim()))
                } else {
                    ToolOutput::err(err.trim().to_string())
                }
            }
            Err(e) => ToolOutput::err(format!("jail {} failed: {}", action, e)),
        }
    }

    async fn exec_in_jail(&self, name: &str, user: &str, command: &str) -> ToolOutput {
        if !Self::is_freebsd() { return Self::not_freebsd(); }
        if name.is_empty() { return ToolOutput::err("name is required"); }
        if command.is_empty() { return ToolOutput::err("command is required"); }

        let output = Self::run_cmd("jexec", &[name, user, command]).output().await;
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
            Err(e) => ToolOutput::err(format!("jexec failed: {}", e)),
        }
    }

    async fn jail_info(&self, name: &str) -> ToolOutput {
        if !Self::is_freebsd() { return Self::not_freebsd(); }

        let output = Self::run_cmd("jls", &["-j", name, "-n"]).output().await;
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
            Err(e) => ToolOutput::err(format!("jls failed: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Tool;

    #[tokio::test]
    async fn test_non_freebsd_guard_list() {
        let tool = JailTool;
        let result = tool.execute(serde_json::json!({"action": "list"})).await;
        assert!(!result.success);
        assert_eq!(result.error.unwrap(), "this tool only works on FreeBSD");
    }

    #[tokio::test]
    async fn test_non_freebsd_guard_info() {
        let tool = JailTool;
        let result = tool.execute(serde_json::json!({"action": "info", "name": "test"})).await;
        assert!(!result.success);
        assert_eq!(result.error.unwrap(), "this tool only works on FreeBSD");
    }

    #[tokio::test]
    async fn test_non_freebsd_guard_exec() {
        let tool = JailTool;
        let result = tool.execute(serde_json::json!({"action": "exec", "name": "test", "command": "ls"})).await;
        assert!(!result.success);
        assert_eq!(result.error.unwrap(), "this tool only works on FreeBSD");
    }

    #[tokio::test]
    async fn test_jail_unknown_action() {
        let tool = JailTool;
        let result = tool.execute(serde_json::json!({"action": "bogus"})).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Unknown action"));
    }

    #[test]
    fn test_jail_input_schema() {
        let tool = JailTool;
        let schema = tool.input_schema();
        assert!(schema["required"].as_array().unwrap().iter().any(|v| v == "action"));
    }
}
