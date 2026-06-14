use anyhow::Result;

use super::client::*;
use super::openai::OpenAIProvider;
use crate::config::LlmConfig;

pub struct OpenRouterProvider(OpenAIProvider);

impl OpenRouterProvider {
    pub fn new(cfg: &LlmConfig) -> Result<Self> {
        Ok(Self(OpenAIProvider::with_defaults(
            cfg,
            "OPENROUTER_API_KEY",
            "https://openrouter.ai/api/v1",
        )?))
    }
}

#[async_trait::async_trait]
impl Provider for OpenRouterProvider {
    fn name(&self) -> &str {
        "openrouter"
    }

    async fn send_message(
        &self,
        system: Option<&str>,
        messages: &[Message],
        tools: &[ToolDef],
    ) -> Result<Message> {
        self.0.send_message(system, messages, tools).await
    }

    async fn stream_message(
        &self,
        system: Option<&str>,
        messages: &[Message],
        tools: &[ToolDef],
        sender: tokio::sync::mpsc::UnboundedSender<StreamEvent>,
    ) {
        self.0.stream_message(system, messages, tools, sender).await
    }
}
