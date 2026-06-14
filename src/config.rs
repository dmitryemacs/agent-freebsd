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
