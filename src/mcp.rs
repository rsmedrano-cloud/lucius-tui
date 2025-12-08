use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
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
    child: Arc<Mutex<Child>>,
    next_id: Arc<Mutex<u64>>,
}

impl Drop for McpClient {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.child.lock() {
            // Try to kill the child; ignore errors
            let _ = guard.kill();
            // Wait for the child to exit
            let _ = guard.wait();
        }
    }
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

pub async fn mcp_manager_task(mcp_client: McpClient, mut request_rx: mpsc::Receiver<McpRequest>) {
    log::info!("MCP manager task started.");
    while let Some(req) = request_rx.recv().await {
        let McpRequest {
            method,
            params,
            response_tx,
        } = req;
        let result = mcp_client.call(&method, params).await;
        if let Err(e) = response_tx.send(result) {
            log::error!("Failed to send MCP response back: {:?}", e);
        }
    }
    log::info!("MCP manager task shutting down.");
}

pub fn parse_tool_call(response: &str) -> Option<ToolCall> {
    lazy_static! {
        static ref TOOL_CALL_REGEX: Regex =
            Regex::new(r"\[TOOL_CALL\]\s*(.*?)\s*\[END_TOOL_CALL\]").unwrap();
    }

    if let Some(captures) = TOOL_CALL_REGEX.captures(response) {
        if let Some(json_str) = captures.get(1) {
            match serde_json::from_str(json_str.as_str()) {
                Ok(tool_call) => return Some(tool_call),
                Err(e) => {
                    log::error!(
                        "Failed to parse tool call JSON: {} from string: {}",
                        e,
                        json_str.as_str()
                    );
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    // no extra imports

    #[test]
    fn test_parse_tool_call_valid() {
        let input = "Some prefix [TOOL_CALL]{\"tool\":\"exec\",\"params\":{\"command\":\"uptime\"}}[END_TOOL_CALL] suffix";
        let call = parse_tool_call(input).expect("Should parse tool call");
        assert_eq!(call.tool, "exec");
        assert!(call.params["command"].as_str().unwrap() == "uptime");
    }

    #[test]
    fn test_parse_tool_call_invalid_json() {
        let input = "[TOOL_CALL]{not_json}[END_TOOL_CALL]";
        let call = parse_tool_call(input);
        assert!(call.is_none());
    }
}

impl McpClient {
    #[allow(dead_code)]
    pub fn new(mcp_server_name: &str) -> Result<Self, std::io::Error> {
        let child = Command::new(mcp_server_name)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        Ok(McpClient {
            child: Arc::new(Mutex::new(child)),
            next_id: Arc::new(Mutex::new(1)),
        })
    }

    // Synchronous blocking call used internally.
    fn call_blocking(
        child_arc: Arc<Mutex<Child>>,
        next_id_arc: Arc<Mutex<u64>>,
        method: &str,
        params: Value,
    ) -> Result<Value, String> {
        let request_id = {
            let mut idlock = next_id_arc
                .lock()
                .map_err(|_| "Failed to lock next_id mutex".to_string())?;
            let id = *idlock;
            *idlock += 1;
            id
        };

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: request_id,
        };

        let mut child_lock = child_arc
            .lock()
            .map_err(|_| "Failed to lock child".to_string())?;
        let stdin = child_lock.stdin.as_mut().ok_or("Failed to open stdin")?;
        let request_json = serde_json::to_string(&request).map_err(|e| e.to_string())? + "\n";

        stdin
            .write_all(request_json.as_bytes())
            .map_err(|e| e.to_string())?;

        let stdout = child_lock.stdout.as_mut().ok_or("Failed to open stdout")?;
        let mut reader = BufReader::new(stdout);
        let mut response_json = String::new();
        reader
            .read_line(&mut response_json)
            .map_err(|e| e.to_string())?;

        let response: JsonRpcResponse =
            serde_json::from_str(&response_json).map_err(|e| e.to_string())?;

        if let Some(error) = response.error {
            return Err(error.to_string());
        }

        response
            .result
            .ok_or_else(|| "No result in response".to_string())
    }

    /// Async wrapper that does not block the tokio worker threads
    pub async fn call(&self, method: &str, params: Value) -> Result<Value, String> {
        let child_arc = self.child.clone();
        let next_id_arc = self.next_id.clone();
        let method_owned = method.to_string();
        let params_owned = params.clone();
        let join_handle = tokio::task::spawn_blocking(move || {
            McpClient::call_blocking(child_arc, next_id_arc, &method_owned, params_owned)
        });
        match join_handle.await {
            Ok(res) => res,
            Err(e) => Err(format!("Failed to join blocking task: {}", e)),
        }
    }
}
