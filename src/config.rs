use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub session: SessionConfig,
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub mcp: McpConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub api_url: Option<String>,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub db_path: String,
    pub max_history: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpConfig {
    #[serde(default)]
    pub servers: Vec<McpServerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub transport: String,
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    pub url: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            llm: LlmConfig::default(),
            session: SessionConfig::default(),
            theme: ThemeConfig::default(),
            mcp: McpConfig::default(),
        }
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            model: "codellama:7b".to_string(),
            api_key: None,
            api_url: Some("http://localhost:11434".to_string()),
            max_tokens: 4096,
            temperature: 0.2,
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            db_path: "~/.local/share/aibsd/sessions.db".to_string(),
            max_history: 100,
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
        }
    }
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let expanded = shellexpand(path);
        let p = Path::new(&expanded);

        if p.exists() {
            let content = std::fs::read_to_string(p)?;
            Ok(toml::from_str(&content)?)
        } else {
            tracing::warn!("config not found at {}, using defaults", path);
            Ok(Config::default())
        }
    }
}

fn shellexpand(s: &str) -> String {
    if s.starts_with("~/") {
        let home = dirs::home_dir().unwrap_or_default();
        home.join(&s[2..]).to_string_lossy().to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.llm.provider, "ollama");
        assert_eq!(cfg.llm.model, "codellama:7b");
        assert_eq!(cfg.llm.max_tokens, 4096);
        assert_eq!(cfg.llm.temperature, 0.2);
        assert_eq!(cfg.session.max_history, 100);
        assert_eq!(cfg.theme.name, "default");
        assert!(cfg.mcp.servers.is_empty());
    }

    #[test]
    fn test_shellexpand_tilde() {
        let expanded = shellexpand("~/.config/aibsd/config.toml");
        assert!(!expanded.starts_with('~'));
        assert!(expanded.ends_with(".config/aibsd/config.toml"));
    }

    #[test]
    fn test_shellexpand_no_tilde() {
        let expanded = shellexpand("/etc/aibsd/config.toml");
        assert_eq!(expanded, "/etc/aibsd/config.toml");
    }

    #[test]
    fn test_toml_roundtrip() {
        let cfg = Config::default();
        let toml_str = toml::to_string(&cfg).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.llm.provider, cfg.llm.provider);
        assert_eq!(parsed.llm.model, cfg.llm.model);
    }

    #[test]
    fn test_toml_with_mcp_server() {
        let toml_str = r#"
[llm]
provider = "anthropic"
model = "claude-sonnet-4-20250514"
max_tokens = 4096
temperature = 0.2

[mcp]
servers = [
  { name = "fs", transport = "stdio", command = "npx", args = ["-y", "mcp-fs"] }
]
"#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.llm.provider, "anthropic");
        assert_eq!(cfg.mcp.servers.len(), 1);
        assert_eq!(cfg.mcp.servers[0].name, "fs");
        assert_eq!(cfg.mcp.servers[0].transport, "stdio");
        assert_eq!(cfg.mcp.servers[0].command.as_deref(), Some("npx"));
        assert_eq!(cfg.mcp.servers[0].args, vec!["-y", "mcp-fs"]);
    }

    #[test]
    fn test_partial_config_uses_defaults() {
        let toml_str = r#"
[llm]
provider = "openai"
model = "gpt-4"
max_tokens = 4096
temperature = 0.2
"#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.llm.provider, "openai");
        assert_eq!(cfg.session.max_history, 100); // default
        assert!(cfg.mcp.servers.is_empty()); // default
    }

    #[test]
    fn test_mcp_server_config_http() {
        let cfg = McpServerConfig {
            name: "remote".to_string(),
            transport: "http".to_string(),
            command: None,
            args: vec![],
            url: Some("http://mcp.example.com".to_string()),
        };
        assert_eq!(cfg.name, "remote");
        assert_eq!(cfg.url.as_deref(), Some("http://mcp.example.com"));
    }
}
