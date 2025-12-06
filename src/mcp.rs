use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use serde_json::Value;
use regex::Regex;
use lazy_static::lazy_static;
use tokio::sync::{mpsc, oneshot};

#[derive(Serialize, Debug)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Value,
    id: u64,
}

#[derive(Deserialize, Debug)]
struct JsonRpcResponse {
    jsonrpc: String,
    result: Option<Value>,
    error: Option<Value>,
    id: u64,
}

#[allow(dead_code)]
pub struct McpClient {
    child: Child,
    next_id: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ToolCall {
    pub tool: String,
    pub params: Value,
}

pub struct McpRequest {
    pub method: String,
    pub params: Value,
    pub response_tx: oneshot::Sender<Result<Value, String>>,
}

pub async fn mcp_manager_task(mut mcp_client: McpClient, mut request_rx: mpsc::Receiver<McpRequest>) {
    log::info!("MCP manager task started.");
    while let Some(req) = request_rx.recv().await {
        let McpRequest { method, params, response_tx } = req;
        let result = mcp_client.call(&method, params);
        if let Err(e) = response_tx.send(result) {
            log::error!("Failed to send MCP response back: {:?}", e);
        }
    }
    log::info!("MCP manager task shutting down.");
}


pub fn parse_tool_call(response: &str) -> Option<ToolCall> {
    lazy_static! {
        static ref TOOL_CALL_REGEX: Regex = Regex::new(r"\[TOOL_CALL\]\s*(.*?)\s*\[END_TOOL_CALL\]").unwrap();
    }

    if let Some(captures) = TOOL_CALL_REGEX.captures(response) {
        if let Some(json_str) = captures.get(1) {
            match serde_json::from_str(json_str.as_str()) {
                Ok(tool_call) => return Some(tool_call),
                Err(e) => {
                    log::error!("Failed to parse tool call JSON: {} from string: {}", e, json_str.as_str());
                }
            }
        }
    }
    None
}

impl McpClient {
    #[allow(dead_code)]
    pub fn new(mcp_server_name: &str) -> Result<Self, std::io::Error> {
        let child = Command::new(mcp_server_name)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        Ok(McpClient {
            child,
            next_id: 1,
        })
    }

    #[allow(dead_code)]
    pub fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: self.next_id,
        };
        self.next_id += 1;

        let stdin = self.child.stdin.as_mut().ok_or("Failed to open stdin")?;
        let request_json = serde_json::to_string(&request).map_err(|e| e.to_string())? + "\n";

        stdin.write_all(request_json.as_bytes()).map_err(|e| e.to_string())?;

        let stdout = self.child.stdout.as_mut().ok_or("Failed to open stdout")?;
        let mut reader = BufReader::new(stdout);
        let mut response_json = String::new();
        reader.read_line(&mut response_json).map_err(|e| e.to_string())?;

        let response: JsonRpcResponse = serde_json::from_str(&response_json).map_err(|e| e.to_string())?;

        if let Some(error) = response.error {
            return Err(error.to_string());
        }

        response.result.ok_or_else(|| "No result in response".to_string())
    }
}
