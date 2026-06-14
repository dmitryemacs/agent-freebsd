mod client;
mod anthropic;
mod openai;
mod ollama;

pub use client::*;
pub use anthropic::AnthropicProvider;
pub use openai::OpenAIProvider;
pub use ollama::OllamaProvider;

use crate::config::LlmConfig;
use anyhow::Result;

pub fn create_provider(cfg: &LlmConfig) -> Result<Box<dyn Provider>> {
    match cfg.provider.to_lowercase().as_str() {
        "anthropic" | "claude" => {
            Ok(Box::new(AnthropicProvider::new(cfg)?))
        }
        "openai" | "gpt" => {
            Ok(Box::new(OpenAIProvider::new(cfg)?))
        }
        "ollama" => {
            Ok(Box::new(OllamaProvider::new(cfg)?))
        }
        _ => anyhow::bail!("unknown provider: {}", cfg.provider),
    }
}
