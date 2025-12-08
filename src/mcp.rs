use serde::{Deserialize, Serialize};
use serde_json::Value;
use regex::Regex;
use lazy_static::lazy_static;

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
#[derive(Serialize, Deserialize, Debug, PartialEq)]
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