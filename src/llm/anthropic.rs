use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::client::*;
use crate::config::LlmConfig;

pub struct AnthropicProvider {
    api_key: String,
    api_url: String,
    model: String,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    temperature: f32,
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
    tools: Vec<ToolDef>,
    stream: bool,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContent>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum AnthropicContent {
    Text { text: String, #[serde(rename = "type")] _type: String },
    ToolUse { id: String, name: String, input: serde_json::Value, #[serde(rename = "type")] _type: String },
    ToolResult { tool_use_id: String, content: String, #[serde(rename = "type")] _type: String },
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct AnthropicResponse {
    content: Vec<AnthropicResponseContent>,
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum AnthropicResponseContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: serde_json::Value },
}

impl AnthropicProvider {
    pub fn new(cfg: &LlmConfig) -> Result<Self> {
        let api_key = cfg.api_key.clone()
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
            .ok_or_else(|| anyhow::anyhow!("ANTHROPIC_API_KEY required"))?;

        Ok(Self {
            api_key,
            api_url: cfg.api_url.clone()
                .unwrap_or_else(|| "https://api.anthropic.com/v1".to_string()),
            model: cfg.model.clone(),
            max_tokens: cfg.max_tokens,
            temperature: cfg.temperature,
        })
    }
}

#[async_trait::async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn send_message(
        &self,
        system: Option<&str>,
        messages: &[Message],
        tools: &[ToolDef],
    ) -> Result<Message> {
        let anthropic_msgs: Vec<AnthropicMessage> = messages.iter().map(|m| {
            let content: Vec<AnthropicContent> = m.content.iter().map(|b| match b {
                ContentBlock::Text { text } => AnthropicContent::Text {
                    text: text.clone(),
                    _type: "text".to_string(),
                },
                ContentBlock::ToolUse { id, name, input } => AnthropicContent::ToolUse {
                    id: id.clone(),
                    name: name.clone(),
                    input: input.clone(),
                    _type: "tool_use".to_string(),
                },
                ContentBlock::ToolResult { tool_use_id, content } => AnthropicContent::ToolResult {
                    tool_use_id: tool_use_id.clone(),
                    content: content.clone(),
                    _type: "tool_result".to_string(),
                },
            }).collect();
            AnthropicMessage {
                role: m.role.to_string(),
                content,
            }
        }).collect();

        let req = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            system: system.map(String::from),
            messages: anthropic_msgs,
            tools: tools.to_vec(),
            stream: false,
        };

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/messages", self.api_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&req)
            .send()
            .await?;

        let anthropic_resp: AnthropicResponse = resp.json().await?;

        let content: Vec<ContentBlock> = anthropic_resp.content.into_iter().map(|c| match c {
            AnthropicResponseContent::Text { text } => ContentBlock::Text { text },
            AnthropicResponseContent::ToolUse { id, name, input } => ContentBlock::ToolUse { id, name, input },
        }).collect();

        Ok(Message {
            role: Role::Assistant,
            content,
        })
    }

    async fn stream_message(
        &self,
        system: Option<&str>,
        messages: &[Message],
        tools: &[ToolDef],
        sender: mpsc::UnboundedSender<StreamEvent>,
    ) {
        // TODO: SSE streaming for Anthropic
        let result = self.send_message(system, messages, tools).await;
        match result {
            Ok(msg) => {
                for block in msg.content {
                    match block {
                        ContentBlock::Text { text } => {
                            let _ = sender.send(StreamEvent::TextDelta(text));
                        }
                        ContentBlock::ToolUse { id, name, input } => {
                            let _ = sender.send(StreamEvent::ToolCallStart(ToolCall {
                                id, name, arguments: input,
                            }));
                        }
                        _ => {}
                    }
                }
                let _ = sender.send(StreamEvent::Done);
            }
            Err(e) => {
                let _ = sender.send(StreamEvent::Error(e.to_string()));
            }
        }
    }
}
