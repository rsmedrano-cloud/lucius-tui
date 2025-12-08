use serde::{Deserialize, Serialize};
use serde_json::Value;
use regex::Regex;
use lazy_static::lazy_static;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use uuid::Uuid;

// --- Task & Tool Data Structures ---

/// Represents a tool call identified from the LLM's output.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ToolCall {
    pub tool: String,
    pub params: Value,
}

/// Represents a task payload to be sent to an mcp-worker via Redis.
#[derive(Serialize, Deserialize, Debug)]
pub struct Task {
    pub id: String,
    // target_host is specified for potential future routing, not currently used by worker.
    pub target_host: String, 
    pub task_type: TaskType,
    pub details: Value,
}

/// The type of task for the worker to execute.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)] // Added Clone for ConfirmationModal
#[serde(rename_all = "UPPERCASE")]
pub enum TaskType {
    DOCKER,
    SHELL,
}

// --- Parsing Logic ---

/// Parses a tool call from the LLM's response string.
/// The format is "[TOOL_CALL] {...} [END_TOOL_CALL]".
pub fn parse_tool_call(response: &str) -> Option<ToolCall> {
    lazy_static! {
        static ref TOOL_CALL_REGEX: Regex = Regex::new(r"\[TOOL_CALL\]\s*(?s)(.*?)\s*\[END_TOOL_CALL\]").unwrap();
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

// --- Redis MCP Interaction Functions ---

pub async fn submit_task(conn: &mut MultiplexedConnection, tool_call: &ToolCall) -> Result<String, String> {
    let task_id = Uuid::new_v4().to_string();
    let task_type = match tool_call.tool.as_str() {
        "exec" | "shell" => TaskType::SHELL,
        "docker" => TaskType::DOCKER,
        _ => TaskType::SHELL, // Default to SHELL for unknown tools
    };

    let task = Task {
        id: task_id.clone(),
        target_host: "any".to_string(), // Target logic can be enhanced later
        task_type,
        details: tool_call.params.clone(),
    };

    let task_json = match serde_json::to_string(&task) {
        Ok(json) => json,
        Err(e) => return Err(format!("Failed to serialize task: {}", e)),
    };

    let queue_key = "mcp::tasks::all";
    
    let rpush_result: redis::RedisResult<()> = conn.rpush(queue_key, &task_json).await;
    match rpush_result {
        Ok(_) => {
            log::info!("Pushed task {} to Redis queue '{}'", task_id, queue_key);
            Ok(task_id)
        },
        Err(e) => Err(format!("Failed to push task to Redis: {}", e)),
    }
}

pub async fn poll_result(conn: &mut MultiplexedConnection, task_id: &str) -> Result<String, String> {
    let result_key = format!("mcp::result::{}", task_id);
    log::info!("Waiting for result on key '{}'", result_key);

    let blpop_result: redis::RedisResult<Vec<String>> = conn.blpop(&result_key, 30.0).await; // 30 second timeout

    match blpop_result {
        Ok(result_vec) => {
            if let Some(result_str) = result_vec.get(1) {
                Ok(result_str.clone())
            } else {
                Err("Received empty result from Redis.".to_string())
            }
        }
        Err(e) => Err(format!("Failed to get result from Redis: {}", e)),
    }
}