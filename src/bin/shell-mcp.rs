use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::process::Command;

#[derive(Deserialize, Debug)]
struct JsonRpcRequest {
    id: Value,
    method: String,
    params: Option<Value>,
}

#[derive(Serialize, Debug)]
struct JsonRpcResponse {
    id: Value,
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

fn main() {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                let response = JsonRpcResponse {
                    id: Value::Null,
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(json!({
                        "code": -32700,
                        "message": format!("Parse error: {}", e),
                    })),
                };
                let response_json = serde_json::to_string(&response).unwrap();
                println!("{}", response_json);
                io::stdout().flush().unwrap();
                continue;
            }
        };

        let response = match request.method.as_str() {
            "list_tools" => handle_list_tools(&request),
            "exec" => handle_exec(&request),
            "remote_exec" => handle_remote_exec(&request),
            _ => JsonRpcResponse {
                id: request.id.clone(),
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(json!({
                    "code": -32601,
                    "message": "Method not found",
                })),
            },
        };

        let response_json = serde_json::to_string(&response).unwrap();
        println!("{}", response_json);
        io::stdout().flush().unwrap();
    }
}

fn handle_list_tools(request: &JsonRpcRequest) -> JsonRpcResponse {
    let tools = json!([
        {
            "name": "exec",
            "description": "Execute a shell command on the local machine.",
            "parameters": {
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute."
                    }
                },
                "required": ["command"]
            }
        },
        {
            "name": "remote_exec",
            "description": "Execute a non-interactive shell command on a remote host via SSH.",
            "parameters": {
                "type": "object",
                "properties": {
                    "host": {
                        "type": "string",
                        "description": "The remote host to connect to, e.g., 'user@hostname'."
                    },
                    "command": {
                        "type": "string",
                        "description": "The command to execute on the remote host."
                    }
                },
                "required": ["host", "command"]
            }
        }
    ]);

    JsonRpcResponse {
        id: request.id.clone(),
        jsonrpc: "2.0".to_string(),
        result: Some(tools),
        error: None,
    }
}

fn handle_exec(request: &JsonRpcRequest) -> JsonRpcResponse {
    let params = match &request.params {
        Some(Value::Object(p)) => p,
        _ => {
            return JsonRpcResponse {
                id: request.id.clone(),
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(json!({
                    "code": -32602,
                    "message": "Invalid params",
                })),
            };
        }
    };

    let command_str = match params.get("command").and_then(|c| c.as_str()) {
        Some(s) => s,
        None => {
            return JsonRpcResponse {
                id: request.id.clone(),
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(json!({
                    "code": -32602,
                    "message": "Missing or invalid 'command' parameter",
                })),
            };
        }
    };

    let output = Command::new("sh")
        .arg("-c")
        .arg(command_str)
        .output();

    match output {
        Ok(out) => {
            let result = json!({
                "stdout": String::from_utf8_lossy(&out.stdout).to_string(),
                "stderr": String::from_utf8_lossy(&out.stderr).to_string(),
                "status": out.status.code(),
            });
            JsonRpcResponse {
                id: request.id.clone(),
                jsonrpc: "2.0".to_string(),
                result: Some(result),
                error: None,
            }
        }
        Err(e) => JsonRpcResponse {
            id: request.id.clone(),
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(json!({
                "code": -32603,
                "message": format!("Failed to execute command: {}", e),
            })),
        },
    }
}

fn handle_remote_exec(request: &JsonRpcRequest) -> JsonRpcResponse {
    let params = match &request.params {
        Some(Value::Object(p)) => p,
        _ => {
            return JsonRpcResponse {
                id: request.id.clone(),
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(json!({ "code": -32602, "message": "Invalid params" })),
            };
        }
    };

    let host = match params.get("host").and_then(|h| h.as_str()) {
        Some(s) => s,
        None => {
            return JsonRpcResponse {
                id: request.id.clone(),
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(json!({ "code": -32602, "message": "Missing 'host' parameter" })),
            };
        }
    };

    let command_str = match params.get("command").and_then(|c| c.as_str()) {
        Some(s) => s,
        None => {
            return JsonRpcResponse {
                id: request.id.clone(),
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(json!({ "code": -32602, "message": "Missing 'command' parameter" })),
            };
        }
    };

    let output = Command::new("ssh")
        .arg(host)
        .arg(command_str)
        .output();

    match output {
        Ok(out) => {
            let result = json!({
                "stdout": String::from_utf8_lossy(&out.stdout).to_string(),
                "stderr": String::from_utf8_lossy(&out.stderr).to_string(),
                "status": out.status.code(),
            });
            JsonRpcResponse {
                id: request.id.clone(),
                jsonrpc: "2.0".to_string(),
                result: Some(result),
                error: None,
            }
        }
        Err(e) => JsonRpcResponse {
            id: request.id.clone(),
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(json!({
                "code": -32603,
                "message": format!("Failed to execute ssh command: {}", e),
            })),
        },
    }
}
