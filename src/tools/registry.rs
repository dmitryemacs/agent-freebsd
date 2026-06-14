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
