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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_delta_conversion() {
        let event: AgentEvent = StreamEvent::TextDelta("hello".to_string()).into();
        match event {
            AgentEvent::TextDelta(t) => assert_eq!(t, "hello"),
            _ => panic!("expected TextDelta"),
        }
    }

    #[test]
    fn test_thinking_delta_conversion() {
        let event: AgentEvent = StreamEvent::ThinkingDelta("thinking...".to_string()).into();
        match event {
            AgentEvent::ThinkingDelta(t) => assert_eq!(t, "thinking..."),
            _ => panic!("expected ThinkingDelta"),
        }
    }

    #[test]
    fn test_tool_call_start_conversion() {
        let tc = ToolCall {
            id: "call_1".to_string(),
            name: "bash".to_string(),
            arguments: serde_json::json!({"command": "ls"}),
        };
        let event: AgentEvent = StreamEvent::ToolCallStart(tc).into();
        match event {
            AgentEvent::ToolCallStart(t) => {
                assert_eq!(t.id, "call_1");
                assert_eq!(t.name, "bash");
            }
            _ => panic!("expected ToolCallStart"),
        }
    }

    #[test]
    fn test_tool_call_end_conversion() {
        let event: AgentEvent = StreamEvent::ToolCallEnd("call_1".to_string()).into();
        match event {
            AgentEvent::ToolCallEnd { id, .. } => assert_eq!(id, "call_1"),
            _ => panic!("expected ToolCallEnd"),
        }
    }

    #[test]
    fn test_done_conversion() {
        let event: AgentEvent = StreamEvent::Done.into();
        assert!(matches!(event, AgentEvent::Done));
    }

    #[test]
    fn test_error_conversion() {
        let event: AgentEvent = StreamEvent::Error("something failed".to_string()).into();
        match event {
            AgentEvent::Error(e) => assert_eq!(e, "something failed"),
            _ => panic!("expected Error"),
        }
    }
}
