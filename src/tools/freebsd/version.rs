use crate::tools::{Tool, ToolOutput};
use serde_json::Value;
use tokio::process::Command;
use std::process::Stdio;

pub struct VersionTool;

#[async_trait::async_trait]
impl Tool for VersionTool {
    fn name(&self) -> &str { "freebsd_version" }
    fn description(&self) -> &str {
        "Get detailed FreeBSD system information: kernel version, userland version, hardware, CPU, memory, uptime, and installed packages count."
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "detail": {
                    "type": "string",
                    "enum": ["full", "kernel", "userland", "hardware", "memory", "uptime", "cpu"],
                    "description": "Level of detail (default: full)",
                    "default": "full"
                }
            }
        })
    }

    async fn execute(&self, args: Value) -> ToolOutput {
        if !Self::is_freebsd() {
            // Return mock info when not on FreeBSD
            return ToolOutput::ok(
                "System: not FreeBSD (development/testing mode)\n\
                 To get actual FreeBSD info, run this tool on a FreeBSD system."
            );
        }

        let detail = args.get("detail")
            .and_then(|v| v.as_str())
            .unwrap_or("full");

        match detail {
            "full" => self.full_info().await,
            "kernel" => self.run("uname", &["-a"]).await,
            "userland" => self.run("freebsd-version", &[]).await,
            "hardware" => self.run("uname", &["-m", "-p"]).await,
            "memory" => self.memory_info().await,
            "uptime" => self.run("uptime", &[]).await,
            "cpu" => self.cpu_info().await,
            _ => ToolOutput::err(format!("Unknown detail: {}", detail)),
        }
    }
}

impl VersionTool {
    fn is_freebsd() -> bool {
        cfg!(target_os = "freebsd")
    }

    async fn run(&self, cmd: &str, args: &[&str]) -> ToolOutput {
        let output = Command::new(cmd)
            .args(args)
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
            Err(e) => ToolOutput::err(format!("{} failed: {}", cmd, e)),
        }
    }

    async fn full_info(&self) -> ToolOutput {
        let kernel = self.run("uname", &["-a"]).await.output;
        let userland = self.run("freebsd-version", &["-k"]).await.output;
        let userland_u = self.run("freebsd-version", &["-u"]).await.output;
        let hw = self.run("uname", &["-m", "-p"]).await.output;
        let uptime = self.run("uptime", &[]).await.output;

        // Get memory info from sysctl
        let mem_total = self.run("sysctl", &["-n", "hw.physmem"]).await.output;
        let mem_pagesize = self.run("sysctl", &["-n", "hw.pagesize"]).await.output;
        let ncpu = self.run("sysctl", &["-n", "hw.ncpu"]).await.output;
        let model = self.run("sysctl", &["-n", "hw.model"]).await.output;

        let info = format!(
            "=== FreeBSD System Information ===\n\
             Kernel: {}\n\
             Kernel version: {}\n\
             Userland version: {}\n\
             Hardware arch: {}\n\
             CPU model: {}\n\
             CPU cores: {}\n\
             Memory (physmem): {} bytes (page size: {})\n\
             Uptime: {}\n\
             =================================",
            kernel.trim(),
            userland.trim(),
            userland_u.trim(),
            hw.trim(),
            model.trim(),
            ncpu.trim(),
            mem_total.trim(),
            mem_pagesize.trim(),
            uptime.trim(),
        );

        ToolOutput::ok(info)
    }

    async fn memory_info(&self) -> ToolOutput {
        let physmem = self.run("sysctl", &["-n", "hw.physmem"]).await.output;
        let realmem = self.run("sysctl", &["-n", "hw.realmem"]).await.output;
        let pagesize = self.run("sysctl", &["-n", "hw.pagesize"]).await.output;

        let phys_gb = physmem.trim().parse::<f64>().unwrap_or(0.0) / 1073741824.0;
        let real_gb = realmem.trim().parse::<f64>().unwrap_or(0.0) / 1073741824.0;

        ToolOutput::ok(format!(
            "Physical memory: {} bytes ({:.2} GB)\n\
             Real memory: {} bytes ({:.2} GB)\n\
             Page size: {} bytes",
            physmem.trim(), phys_gb,
            realmem.trim(), real_gb,
            pagesize.trim(),
        ))
    }

    async fn cpu_info(&self) -> ToolOutput {
        let model = self.run("sysctl", &["-n", "hw.model"]).await.output;
        let ncpu = self.run("sysctl", &["-n", "hw.ncpu"]).await.output;
        let features = self.run("sysctl", &["-n", "hw.instruction_sets"]).await.output;

        ToolOutput::ok(format!(
            "CPU model: {}\n\
             Cores: {}\n\
             Instruction sets: {}",
            model.trim(),
            ncpu.trim(),
            features.trim(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Tool;

    #[tokio::test]
    async fn test_non_freebsd_returns_mock() {
        let tool = VersionTool;
        let result = tool.execute(serde_json::json!({})).await;
        assert!(result.success);
        assert!(result.output.contains("not FreeBSD"));
    }

    #[tokio::test]
    async fn test_version_unknown_detail() {
        let tool = VersionTool;
        let result = tool.execute(serde_json::json!({"detail": "bogus"})).await;
        if cfg!(target_os = "freebsd") {
            assert!(!result.success);
            assert!(result.error.unwrap().contains("Unknown detail"));
        }
    }

    #[test]
    fn test_version_input_schema() {
        let tool = VersionTool;
        let schema = tool.input_schema();
        assert!(schema.get("properties").is_some());
    }
}
