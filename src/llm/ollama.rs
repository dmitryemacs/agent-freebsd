use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::client::*;
use crate::config::LlmConfig;

pub struct OllamaProvider {
    api_url: String,
    model: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: u32,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ChatResponse {
    message: OllamaResponseMessage,
    done: bool,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct OllamaResponseMessage {
    role: String,
    content: String,
}

impl OllamaProvider {
    pub fn new(cfg: &LlmConfig) -> Result<Self> {
        Ok(Self {
            api_url: cfg.api_url.clone()
                .unwrap_or_else(|| "http://localhost:11434".to_string()),
            model: cfg.model.clone(),
        })
    }

    fn convert_messages(&self, system: Option<&str>, messages: &[Message]) -> Vec<OllamaMessage> {
        let mut result = Vec::new();

        if let Some(sys) = system {
            result.push(OllamaMessage {
                role: "system".to_string(),
                content: sys.to_string(),
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

            result.push(OllamaMessage {
                role: msg.role.to_string(),
                content,
            });
        }

        result
    }
}

#[async_trait::async_trait]
impl Provider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    async fn send_message(
        &self,
        system: Option<&str>,
        messages: &[Message],
        _tools: &[ToolDef],
    ) -> Result<Message> {
        let client = reqwest::Client::new();
        let req = ChatRequest {
            model: self.model.clone(),
            messages: self.convert_messages(system, messages),
            stream: false,
            options: OllamaOptions {
                temperature: 0.2,
                num_predict: 4096,
            },
        };

        let resp = client
            .post(format!("{}/api/chat", self.api_url))
            .json(&req)
            .send()
            .await?;

        let chat_resp: ChatResponse = resp.json().await?;

        Ok(Message {
            role: Role::Assistant,
            content: vec![ContentBlock::Text {
                text: chat_resp.message.content,
            }],
        })
    }

    async fn stream_message(
        &self,
        system: Option<&str>,
        messages: &[Message],
        _tools: &[ToolDef],
        sender: mpsc::UnboundedSender<StreamEvent>,
    ) {
        let client = reqwest::Client::new();
        let req = ChatRequest {
            model: self.model.clone(),
            messages: self.convert_messages(system, messages),
            stream: true,
            options: OllamaOptions {
                temperature: 0.2,
                num_predict: 4096,
            },
        };

        match client
            .post(format!("{}/api/chat", self.api_url))
            .json(&req)
            .send()
            .await
        {
            Ok(resp) => {
                let mut stream = resp.bytes_stream();
                use futures_util::StreamExt;
                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(bytes) => {
                            if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                                for line in text.lines() {
                                    if line.is_empty() { continue; }
                                    if let Ok(part) = serde_json::from_str::<ChatResponse>(line) {
                                        let _ = sender.send(StreamEvent::TextDelta(part.message.content));
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let _ = sender.send(StreamEvent::Error(e.to_string()));
                            return;
                        }
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
