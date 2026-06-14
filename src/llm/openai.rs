use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::client::*;
use crate::config::LlmConfig;

pub struct OpenAIProvider {
    api_key: String,
    api_url: String,
    model: String,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    tools: Vec<OpenAITool>,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
}

#[derive(Serialize)]
struct OpenAIMessage {
    role: String,
    content: String,
    tool_calls: Option<Vec<OpenAIToolCall>>,
    tool_call_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct OpenAIToolCall {
    id: String,
    #[serde(rename = "type")]
    _type: String,
    function: OpenAIFunction,
}

#[derive(Serialize, Deserialize)]
struct OpenAIFunction {
    name: String,
    arguments: String,
}

#[derive(Serialize)]
struct OpenAITool {
    #[serde(rename = "type")]
    _type: String,
    function: OpenAIToolFunction,
}

#[derive(Serialize)]
struct OpenAIToolFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct OpenAIChoice {
    message: OpenAIResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct OpenAIResponseMessage {
    role: String,
    content: Option<String>,
    tool_calls: Option<Vec<OpenAIToolCall>>,
}

impl OpenAIProvider {
    pub fn new(cfg: &LlmConfig) -> Result<Self> {
        let api_key = cfg.api_key.clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| anyhow::anyhow!("OPENAI_API_KEY required"))?;

        Ok(Self {
            api_key,
            api_url: cfg.api_url.clone()
                .unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            model: cfg.model.clone(),
            max_tokens: cfg.max_tokens,
            temperature: cfg.temperature,
        })
    }
}

#[async_trait::async_trait]
impl Provider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }

    async fn send_message(
        &self,
        system: Option<&str>,
        messages: &[Message],
        tools: &[ToolDef],
    ) -> Result<Message> {
        let mut openai_msgs = Vec::new();

        if let Some(sys) = system {
            openai_msgs.push(OpenAIMessage {
                role: "system".to_string(),
                content: sys.to_string(),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        for msg in messages {
            let content = msg.content.iter()
                .map(|b| match b {
                    ContentBlock::Text { text } => text.clone(),
                    _ => String::new(),
                })
                .collect::<Vec<_>>()
                .join("\n");

            openai_msgs.push(OpenAIMessage {
                role: msg.role.to_string(),
                content,
                tool_calls: None,
                tool_call_id: None,
            });
        }

        let openai_tools: Vec<OpenAITool> = tools.iter().map(|t| OpenAITool {
            _type: "function".to_string(),
            function: OpenAIToolFunction {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: t.input_schema.clone(),
            },
        }).collect();

        let req = OpenAIRequest {
            model: self.model.clone(),
            messages: openai_msgs,
            tools: openai_tools,
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            stream: false,
        };

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/chat/completions", self.api_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&req)
            .send()
            .await?;

        let oai_resp: OpenAIResponse = resp.json().await?;

        if let Some(choice) = oai_resp.choices.into_iter().next() {
            let mut content = Vec::new();

            if let Some(text) = choice.message.content {
                if !text.is_empty() {
                    content.push(ContentBlock::Text { text });
                }
            }

            if let Some(tcs) = choice.message.tool_calls {
                for tc in tcs {
                    content.push(ContentBlock::ToolUse {
                        id: tc.id,
                        name: tc.function.name,
                        input: serde_json::from_str(&tc.function.arguments)
                            .unwrap_or(serde_json::Value::Null),
                    });
                }
            }

            return Ok(Message {
                role: Role::Assistant,
                content,
            });
        }

        Ok(Message {
            role: Role::Assistant,
            content: vec![],
        })
    }

    async fn stream_message(
        &self,
        system: Option<&str>,
        messages: &[Message],
        tools: &[ToolDef],
        sender: mpsc::UnboundedSender<StreamEvent>,
    ) {
        // Non-streaming fallback for now
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
