use crate::tools::{Tool, ToolOutput};
use anyhow::Result;
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};

use super::transport::McpTransport;
use super::types::*;

pub struct McpClient {
    transport: Box<dyn McpTransport>,
    next_id: AtomicU64,
    server_name: tokio::sync::Mutex<String>,
}

impl McpClient {
    pub fn new(transport: Box<dyn McpTransport>) -> Self {
        Self {
            transport,
            next_id: AtomicU64::new(1),
            server_name: tokio::sync::Mutex::new(String::new()),
        }
    }

    pub async fn server_name(&self) -> String {
        self.server_name.lock().await.clone()
    }

    async fn send_request(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        let msg = serde_json::to_string(&req)?;
        let resp = self.transport.request(&msg).await?;
        let response: JsonRpcResponse = serde_json::from_str(&resp)?;

        if let Some(err) = response.error {
            anyhow::bail!("MCP {} error: {} (code {})", method, err.message, err.code);
        }

        response.result.ok_or_else(|| anyhow::anyhow!("MCP {} returned no result", method))
    }

    pub async fn initialize(&self) -> Result<()> {
        let params = InitializeParams {
            protocol_version: "2025-03-26".to_string(),
            capabilities: ClientCapabilities {
                tools: Some(Value::Object(serde_json::Map::new())),
                resources: None,
            },
            client_info: ClientInfo {
                name: "aibsd".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        let result = self.send_request(
            "initialize",
            Some(serde_json::to_value(params)?),
        ).await?;

        let init_result: InitializeResult = serde_json::from_value(result)?;
        *self.server_name.lock().await = init_result.server_info.name;

        // Send initialized notification (no response expected)
        let notification = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 0,
            method: "notifications/initialized".to_string(),
            params: None,
        };
        let msg = serde_json::to_string(&notification)?;
        let _ = self.transport.request(&msg).await;

        Ok(())
    }

    pub async fn list_tools(&self) -> Result<Vec<McpTool>> {
        let result = self.send_request("tools/list", Some(Value::Object(serde_json::Map::new()))).await?;
        let list: ListToolsResult = serde_json::from_value(result)?;
        Ok(list.tools)
    }

    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<String> {
        let params = CallToolParams {
            name: name.to_string(),
            arguments,
        };

        let result = self.send_request("tools/call", Some(serde_json::to_value(params)?)).await?;
        let call_result: CallToolResult = serde_json::from_value(result)?;

        let text: Vec<String> = call_result.content.iter()
            .filter_map(|c| match c {
                ToolContent::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect();

        Ok(text.join("\n"))
    }
}

// Tool adapter — wraps an MCP tool as a local Tool
pub struct McpToolAdapter {
    client: std::sync::Arc<tokio::sync::Mutex<McpClient>>,
    mcp_tool: McpTool,
}

impl McpToolAdapter {
    pub fn new(client: std::sync::Arc<tokio::sync::Mutex<McpClient>>, mcp_tool: McpTool) -> Self {
        Self { client, mcp_tool }
    }
}

#[async_trait::async_trait]
impl Tool for McpToolAdapter {
    fn name(&self) -> &str {
        &self.mcp_tool.name
    }

    fn description(&self) -> &str {
        self.mcp_tool.description.as_deref().unwrap_or("")
    }

    fn input_schema(&self) -> Value {
        self.mcp_tool.inputSchema.clone()
    }

    async fn execute(&self, args: Value) -> ToolOutput {
        let client = self.client.lock().await;
        match client.call_tool(&self.mcp_tool.name, args).await {
            Ok(result) => ToolOutput::ok(result),
            Err(e) => ToolOutput::err(format!("MCP tool error: {}", e)),
        }
    }
}
