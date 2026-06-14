use crate::config::Config;
use crate::llm::{Message, Role, ContentBlock, StreamEvent, Provider};
use crate::tools;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;

use super::AgentEvent;

#[allow(dead_code)]
pub struct Agent {
    provider: Arc<dyn Provider>,
    tools: tools::registry::Registry,
    config: Config,
    messages: Vec<Message>,
}

impl Agent {
    pub fn new(
        provider: Box<dyn Provider>,
        tools: tools::registry::Registry,
        config: &Config,
    ) -> Self {
        Self {
            provider: Arc::from(provider),
            tools,
            config: config.clone(),
            messages: Vec::new(),
        }
    }

    fn system_prompt(&self) -> String {
        let tool_descriptions: Vec<String> = self.tools.definitions().iter()
            .map(|t| format!("  - {}: {}", t.name, t.description))
            .collect();

        let _os = std::env::consts::OS;
        let on_freebsd = cfg!(target_os = "freebsd");
        let os_note = if on_freebsd {
            "You are running on FreeBSD. All FreeBSD-specific tools are available."
        } else {
            "You are NOT running on FreeBSD. FreeBSD-specific tools will report errors if called."
        };

        format!(r#"You are aibsd, a FreeBSD-first AI coding agent.

You have access to the following tools:
{}

Rules:
1. Think step by step before using tools.
2. Use bash for running commands, compilation, package installation.
3. Use read/write/edit to work with files.
4. Use glob/grep to explore the codebase.
5. For FreeBSD system administration, always prefer the dedicated tools
   (freebsd_jail, freebsd_zfs, freebsd_pkg, freebsd_service, freebsd_pf,
    freebsd_ports, freebsd_build, freebsd_sysctl, freebsd_dtrace, freebsd_version)
   over raw bash commands.
6. Check if a command is FreeBSD-compatible before suggesting it.
7. Use ZFS-native tools for storage management.
8. Prefer pkg over make install for package management.

{}"#,
            tool_descriptions.join("\n"),
            os_note,
        )
    }

    pub async fn run(&mut self, prompt: &str, event_tx: Option<mpsc::UnboundedSender<AgentEvent>>) -> Result<String> {
        self.messages.push(Message {
            role: Role::User,
            content: vec![ContentBlock::Text { text: prompt.to_string() }],
        });

        let max_iterations = 20;
        let mut full_response = String::new();

        for _iteration in 0..max_iterations {
            let (stream_tx, mut stream_rx) = mpsc::unbounded_channel::<StreamEvent>();

            let tool_defs = self.tools.definitions();
            let provider = Arc::clone(&self.provider);
            let system = self.system_prompt();
            let msgs = self.messages.clone();

            tokio::spawn(async move {
                provider.stream_message(
                    Some(&system),
                    &msgs,
                    &tool_defs,
                    stream_tx,
                ).await;
            });

            let mut response_text = String::new();
            let mut pending_tool_calls = Vec::new();

            while let Some(stream_event) = stream_rx.recv().await {
                let agent_event: AgentEvent = stream_event.clone().into();
                if let Some(ref tx) = event_tx {
                    let _ = tx.send(agent_event);
                }

                match stream_event {
                    StreamEvent::TextDelta(text) => {
                        response_text.push_str(&text);
                    }
                    StreamEvent::ToolCallStart(tc) => {
                        pending_tool_calls.push(tc);
                    }
                    StreamEvent::Error(err) => {
                        eprintln!("Error: {}", err);
                    }
                    StreamEvent::Done => break,
                    _ => {}
                }
            }

            if pending_tool_calls.is_empty() {
                self.messages.push(Message {
                    role: Role::Assistant,
                    content: vec![ContentBlock::Text { text: response_text.clone() }],
                });
                full_response = response_text;
                break;
            }

            let mut assistant_content = Vec::new();
            if !response_text.is_empty() {
                assistant_content.push(ContentBlock::Text { text: response_text.clone() });
            }

            for tc in &pending_tool_calls {
                assistant_content.push(ContentBlock::ToolUse {
                    id: tc.id.clone(),
                    name: tc.name.clone(),
                    input: tc.arguments.clone(),
                });
            }

            self.messages.push(Message {
                role: Role::Assistant,
                content: assistant_content,
            });

            for tc in &pending_tool_calls {
                let result = match self.tools.get(&tc.name) {
                    Some(tool) => {
                        tool.execute(tc.arguments.clone()).await
                    }
                    None => tools::ToolOutput::err(format!("Unknown tool: {}", tc.name)),
                };

                let result_str = result.to_string();

                if let Some(ref tx) = event_tx {
                    let _ = tx.send(AgentEvent::ToolCallEnd {
                        id: tc.id.clone(),
                        name: tc.name.clone(),
                        result: result_str.clone(),
                    });
                }

                self.messages.push(Message {
                    role: Role::Tool,
                    content: vec![ContentBlock::ToolResult {
                        tool_use_id: tc.id.clone(),
                        content: result_str,
                    }],
                });
            }

            full_response = response_text;
        }

        Ok(full_response)
    }

    #[allow(dead_code)]
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }
}
