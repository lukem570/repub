//! MCP server implementation.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

use crate::error::{McpError, McpResult};
use crate::protocol::*;
use crate::resources;
use crate::session::Session;
use crate::tools::{self, Tool};

/// MCP server for Repub.
pub struct McpServer {
    /// Current session
    session: Session,
    /// Registered tools
    tools: HashMap<String, Box<dyn Tool>>,
    /// Whether the server has been initialized
    initialized: bool,
}

impl McpServer {
    /// Create a new MCP server.
    pub fn new() -> Self {
        let mut tools_map = HashMap::new();
        for tool in tools::all_tools() {
            let def = tool.definition();
            tools_map.insert(def.name.clone(), tool);
        }

        Self {
            session: Session::new(),
            tools: tools_map,
            initialized: false,
        }
    }

    /// Run the server on stdio (synchronous version).
    pub fn run_stdio_sync(&mut self) -> McpResult<()> {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();
        let reader = BufReader::new(stdin.lock());

        for line in reader.lines() {
            let line = line?;
            if line.is_empty() {
                continue;
            }

            match self.handle_message(&line) {
                Ok(Some(response)) => {
                    let response_json = serde_json::to_string(&response)?;
                    writeln!(stdout, "{}", response_json)?;
                    stdout.flush()?;
                }
                Ok(None) => {
                    // No response needed (notification)
                }
                Err(e) => {
                    // Log error but continue
                    eprintln!("Error handling message: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Run the server on stdio (async version).
    pub async fn run_stdio(&mut self) -> McpResult<()> {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let reader = tokio::io::BufReader::new(stdin);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            if line.is_empty() {
                continue;
            }

            match self.handle_message(&line) {
                Ok(Some(response)) => {
                    let response_json = serde_json::to_string(&response)?;
                    stdout.write_all(response_json.as_bytes()).await?;
                    stdout.write_all(b"\n").await?;
                    stdout.flush().await?;
                }
                Ok(None) => {}
                Err(e) => {
                    eprintln!("Error handling message: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Handle a single message.
    fn handle_message(&mut self, message: &str) -> McpResult<Option<Response>> {
        let request: Request = match serde_json::from_str(message) {
            Ok(r) => r,
            Err(e) => {
                return Ok(Some(Response::error(
                    RequestId::Null,
                    RpcError::parse_error(e.to_string()),
                )));
            }
        };

        let result = self.dispatch(&request);

        match result {
            Ok(value) => Ok(Some(Response::success(request.id, value))),
            Err(e) => Ok(Some(Response::error(
                request.id,
                RpcError::internal_error(e.to_string()),
            ))),
        }
    }

    /// Dispatch a request to the appropriate handler.
    fn dispatch(&mut self, request: &Request) -> McpResult<Value> {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request.params.clone()),
            "initialized" => {
                self.initialized = true;
                Ok(json!({}))
            }
            "tools/list" => self.handle_list_tools(),
            "tools/call" => self.handle_call_tool(request.params.clone()),
            "resources/list" => self.handle_list_resources(),
            "resources/read" => self.handle_read_resource(request.params.clone()),
            "ping" => Ok(json!({})),
            _ => Err(McpError::Protocol(format!("Unknown method: {}", request.method))),
        }
    }

    /// Handle initialize request.
    fn handle_initialize(&self, _params: Option<Value>) -> McpResult<Value> {
        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability::default()),
                resources: Some(ResourcesCapability::default()),
                prompts: None,
            },
            server_info: ServerInfo {
                name: "repub".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        Ok(serde_json::to_value(result)?)
    }

    /// Handle tools/list request.
    fn handle_list_tools(&self) -> McpResult<Value> {
        let tools: Vec<ToolDefinition> = self.tools.values()
            .map(|t| t.definition())
            .collect();

        Ok(json!({
            "tools": tools
        }))
    }

    /// Handle tools/call request.
    fn handle_call_tool(&mut self, params: Option<Value>) -> McpResult<Value> {
        let params = params.ok_or_else(|| McpError::InvalidArgument("Missing params".to_string()))?;
        
        let name = params["name"].as_str()
            .ok_or_else(|| McpError::InvalidArgument("Missing tool name".to_string()))?
            .to_string();
        
        let args = params.get("arguments").cloned().unwrap_or(json!({}));

        // Get the tool and execute it
        let tool = self.tools.get(&name)
            .ok_or_else(|| McpError::UnknownTool(name.clone()))?;
        
        let result = tool.execute(&mut self.session, args)?;

        Ok(serde_json::to_value(result)?)
    }

    /// Handle resources/list request.
    fn handle_list_resources(&self) -> McpResult<Value> {
        let resources = resources::list_resources();
        Ok(json!({
            "resources": resources
        }))
    }

    /// Handle resources/read request.
    fn handle_read_resource(&self, params: Option<Value>) -> McpResult<Value> {
        let params = params.ok_or_else(|| McpError::InvalidArgument("Missing params".to_string()))?;
        
        let uri = params["uri"].as_str()
            .ok_or_else(|| McpError::InvalidArgument("Missing resource URI".to_string()))?;

        let content = resources::read_resource(&self.session, uri)?;
        
        Ok(json!({
            "contents": [content]
        }))
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let server = McpServer::new();
        assert!(!server.tools.is_empty());
    }

    #[test]
    fn test_initialize() {
        let server = McpServer::new();
        let result = server.handle_initialize(None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_tools() {
        let server = McpServer::new();
        let result = server.handle_list_tools();
        assert!(result.is_ok());
        
        let value = result.unwrap();
        assert!(value["tools"].is_array());
        assert!(!value["tools"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_call_tool_create_document() {
        let mut server = McpServer::new();
        
        let result = server.handle_call_tool(Some(json!({
            "name": "repub_create_document",
            "arguments": {
                "size": "letter",
                "pages": 2
            }
        })));
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_message_handling() {
        let mut server = McpServer::new();
        
        let request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let response = server.handle_message(request);
        
        assert!(response.is_ok());
        assert!(response.unwrap().is_some());
    }
}
