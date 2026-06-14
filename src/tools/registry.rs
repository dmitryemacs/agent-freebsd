use std::collections::HashMap;
use crate::tools::Tool;
use crate::tools::core::{ReadTool, WriteTool, BashTool, GlobTool, GrepTool, EditTool};
use crate::tools::freebsd::{
    JailTool, ZfsTool, PkgTool, ServiceTool, PfTool,
    PortsTool, BuildTool, SysctlTool, DTraceTool, VersionTool,
};

pub struct Registry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    pub fn definitions(&self) -> Vec<crate::llm::ToolDef> {
        self.tools.values().map(|t| {
            crate::llm::ToolDef {
                name: t.name().to_string(),
                description: t.description().to_string(),
                input_schema: t.input_schema(),
            }
        }).collect()
    }

    #[allow(dead_code)]
    pub fn list(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn builtin_tools() -> Registry {
    let mut r = Registry::new();

    // Core tools
    r.register(Box::new(ReadTool));
    r.register(Box::new(WriteTool));
    r.register(Box::new(EditTool));
    r.register(Box::new(BashTool));
    r.register(Box::new(GlobTool));
    r.register(Box::new(GrepTool));

    // FreeBSD tools
    r.register(Box::new(JailTool));
    r.register(Box::new(ZfsTool));
    r.register(Box::new(PkgTool));
    r.register(Box::new(ServiceTool));
    r.register(Box::new(PfTool));
    r.register(Box::new(PortsTool));
    r.register(Box::new(BuildTool));
    r.register(Box::new(SysctlTool));
    r.register(Box::new(DTraceTool));
    r.register(Box::new(VersionTool));

    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolOutput;

    #[test]
    fn test_registry_empty() {
        let r = Registry::new();
        assert!(r.get("read").is_none());
        assert!(r.definitions().is_empty());
    }

    #[test]
    fn test_register_and_get() {
        let mut r = Registry::new();
        r.register(Box::new(ReadTool));
        let tool = r.get("read");
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().name(), "read");
    }

    #[test]
    fn test_register_overwrite() {
        let mut r = Registry::new();
        r.register(Box::new(ReadTool));
        r.register(Box::new(ReadTool));
        assert!(r.get("read").is_some());
    }

    #[test]
    fn test_builtin_tools_count() {
        let r = builtin_tools();
        // 6 core + 10 freebsd = 16
        assert_eq!(r.definitions().len(), 16);
    }

    #[test]
    fn test_builtin_tools_all_present() {
        let r = builtin_tools();
        let names: Vec<&str> = r.list();
        assert!(names.contains(&"read"));
        assert!(names.contains(&"bash"));
        assert!(names.contains(&"freebsd_jail"));
        assert!(names.contains(&"freebsd_zfs"));
        assert!(names.contains(&"freebsd_pkg"));
        assert!(names.contains(&"freebsd_version"));
    }

    #[test]
    fn test_definitions_format() {
        let mut r = Registry::new();

        struct DummyTool;
        #[async_trait::async_trait]
        impl Tool for DummyTool {
            fn name(&self) -> &str { "dummy" }
            fn description(&self) -> &str { "a dummy tool" }
            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({"type": "object"})
            }
            async fn execute(&self, _args: serde_json::Value) -> ToolOutput {
                ToolOutput::ok("ok")
            }
        }

        r.register(Box::new(DummyTool));
        let defs = r.definitions();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].name, "dummy");
        assert_eq!(defs[0].description, "a dummy tool");
    }

    #[test]
    fn test_get_unknown_tool() {
        let r = Registry::new();
        assert!(r.get("nonexistent").is_none());
    }
}
