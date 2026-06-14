use crate::llm::{StreamEvent, ToolCall};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AgentEvent {
    TextDelta(String),
    ThinkingDelta(String),
    ToolCallStart(ToolCall),
    ToolCallEnd {
        id: String,
        name: String,
        result: String,
    },
    TurnComplete {
        response: String,
    },
    Error(String),
    Done,
}

impl From<StreamEvent> for AgentEvent {
    fn from(e: StreamEvent) -> Self {
        match e {
            StreamEvent::TextDelta(t) => AgentEvent::TextDelta(t),
            StreamEvent::ThinkingDelta(t) => AgentEvent::ThinkingDelta(t),
            StreamEvent::ToolCallStart(tc) => AgentEvent::ToolCallStart(tc),
            StreamEvent::ToolCallEnd(id) => AgentEvent::ToolCallEnd {
                id,
                name: String::new(),
                result: String::new(),
            },
            StreamEvent::Done => AgentEvent::Done,
            StreamEvent::Error(e) => AgentEvent::Error(e),
        }
    }
}
