mod client;
mod anthropic;
mod openai;
mod ollama;
mod openrouter;

pub use client::*;
pub use anthropic::AnthropicProvider;
pub use openai::OpenAIProvider;
pub use ollama::OllamaProvider;
pub use openrouter::OpenRouterProvider;

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
        "openrouter" => {
            Ok(Box::new(OpenRouterProvider::new(cfg)?))
        }
        _ => anyhow::bail!("unknown provider: {}", cfg.provider),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_provider_ollama() {
        let cfg = LlmConfig {
            provider: "ollama".to_string(),
            model: "codellama:7b".to_string(),
            api_key: None,
            api_url: Some("http://localhost:11434".to_string()),
            max_tokens: 4096,
            temperature: 0.2,
        };
        let provider = create_provider(&cfg).unwrap();
        assert_eq!(provider.name(), "ollama");
    }

    #[test]
    fn test_create_provider_aliases() {
        let cases = [
            ("anthropic", "anthropic"),
            ("claude", "anthropic"),
            ("openai", "openai"),
            ("gpt", "openai"),
            ("openrouter", "openrouter"),
        ];

        for (alias, expected) in &cases {
            let cfg = LlmConfig {
                provider: alias.to_string(),
                model: "test".to_string(),
                api_key: Some("sk-test".to_string()),
                api_url: Some("http://test".to_string()),
                max_tokens: 100,
                temperature: 0.5,
            };
            let provider = create_provider(&cfg).unwrap();
            assert_eq!(provider.name(), *expected, "alias '{}' should map to '{}'", alias, expected);
        }
    }

    #[test]
    fn test_create_provider_unknown() {
        let cfg = LlmConfig {
            provider: "nonexistent".to_string(),
            model: "test".to_string(),
            api_key: None,
            api_url: None,
            max_tokens: 100,
            temperature: 0.5,
        };
        let result = create_provider(&cfg);
        assert!(result.is_err());
    }

    #[test]
    fn test_openai_provider_defaults() {
        let provider = OpenAIProvider::with_defaults(
            &LlmConfig {
                provider: "openai".to_string(),
                model: "gpt-4".to_string(),
                api_key: Some("sk-test".to_string()),
                api_url: None,
                max_tokens: 100,
                temperature: 0.5,
            },
            "OPENAI_API_KEY",
            "https://api.openai.com/v1",
        ).unwrap();
        assert_eq!(provider.name(), "openai");
    }

    #[test]
    fn test_openai_provider_missing_key() {
        let result = OpenAIProvider::with_defaults(
            &LlmConfig {
                provider: "openai".to_string(),
                model: "gpt-4".to_string(),
                api_key: None,
                api_url: None,
                max_tokens: 100,
                temperature: 0.5,
            },
            "MISSING_ENV_VAR_FOR_TEST",
            "https://api.test.com/v1",
        );
        assert!(result.is_err());
    }
}
