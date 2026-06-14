use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

// MCP Initialize
#[derive(Debug, Serialize, Deserialize)]
pub struct InitializeParams {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    #[serde(rename = "clientInfo")]
    pub client_info: ClientInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitializeResult {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

// MCP Tools
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub inputSchema: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListToolsResult {
    pub tools: Vec<McpTool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CallToolParams {
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct CallToolResult {
    pub content: Vec<ToolContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isError: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "resource")]
    Resource { resource: Value },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc_request_serde() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "tools/list".to_string(),
            params: Some(serde_json::json!({})),
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: JsonRpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.jsonrpc, "2.0");
        assert_eq!(back.id, 1);
        assert_eq!(back.method, "tools/list");
    }

    #[test]
    fn test_json_rpc_response_ok() {
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: 1,
            result: Some(serde_json::json!({"tools": []})),
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: JsonRpcResponse = serde_json::from_str(&json).unwrap();
        assert!(back.error.is_none());
        assert!(back.result.is_some());
    }

    #[test]
    fn test_json_rpc_response_error() {
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: 1,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: "Method not found".to_string(),
                data: None,
            }),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: JsonRpcResponse = serde_json::from_str(&json).unwrap();
        let err = back.error.unwrap();
        assert_eq!(err.code, -32601);
        assert_eq!(err.message, "Method not found");
    }

    #[test]
    fn test_initialize_params_serde() {
        let params = InitializeParams {
            protocol_version: "2025-03-26".to_string(),
            capabilities: ClientCapabilities {
                tools: Some(serde_json::json!({})),
                resources: None,
            },
            client_info: ClientInfo {
                name: "aibsd".to_string(),
                version: "0.1.0".to_string(),
            },
        };
        let json = serde_json::to_string(&params).unwrap();
        let back: InitializeParams = serde_json::from_str(&json).unwrap();
        assert_eq!(back.protocol_version, "2025-03-26");
        assert_eq!(back.client_info.name, "aibsd");
    }

    #[test]
    fn test_initialize_result_serde() {
        let result = InitializeResult {
            protocol_version: "2025-03-26".to_string(),
            capabilities: ServerCapabilities {
                tools: Some(serde_json::json!({})),
                resources: None,
            },
            server_info: ServerInfo {
                name: "test-server".to_string(),
                version: "1.0.0".to_string(),
            },
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: InitializeResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.server_info.name, "test-server");
    }

    #[test]
    fn test_mcp_tool_serde() {
        let tool = McpTool {
            name: "read_file".to_string(),
            description: Some("Read a file".to_string()),
            inputSchema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                }
            }),
        };
        let json = serde_json::to_string(&tool).unwrap();
        let back: McpTool = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "read_file");
        assert_eq!(back.description.as_deref(), Some("Read a file"));
    }

    #[test]
    fn test_list_tools_result_serde() {
        let result = ListToolsResult {
            tools: vec![
                McpTool {
                    name: "tool1".to_string(),
                    description: None,
                    inputSchema: serde_json::json!({}),
                },
            ],
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: ListToolsResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.tools.len(), 1);
        assert_eq!(back.tools[0].name, "tool1");
    }

    #[test]
    fn test_call_tool_params_serde() {
        let params = CallToolParams {
            name: "bash".to_string(),
            arguments: serde_json::json!({"command": "ls"}),
        };
        let json = serde_json::to_string(&params).unwrap();
        let back: CallToolParams = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "bash");
        assert_eq!(back.arguments["command"], "ls");
    }

    #[test]
    fn test_call_tool_result_text() {
        let result = CallToolResult {
            content: vec![ToolContent::Text { text: "hello".to_string() }],
            isError: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: CallToolResult = serde_json::from_str(&json).unwrap();
        match &back.content[0] {
            ToolContent::Text { text } => assert_eq!(text, "hello"),
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn test_call_tool_result_is_error() {
        let result = CallToolResult {
            content: vec![ToolContent::Text { text: "fail".to_string() }],
            isError: Some(true),
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: CallToolResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.isError, Some(true));
    }
}
