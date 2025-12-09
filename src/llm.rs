use serde::Deserialize;
use crate::mcp::{parse_tool_call, ToolCall};

#[derive(Deserialize, Clone)]
pub struct Model {
    pub name: String,
}

#[derive(Deserialize)]
pub struct TagsResponse {
    pub models: Vec<Model>,
}

#[derive(PartialEq)] // Added for comparison in ConfirmationModal
pub enum LLMResponse {
    FinalResponse(String),
    ToolCallDetected(ToolCall),
}

pub async fn ping_ollama(url: String) -> bool {
    let client = reqwest::Client::new();
    let res = client.get(url).send().await;
    res.is_ok()
}

pub async fn fetch_models(url: String) -> Result<Vec<Model>, reqwest::Error> {
    let client = reqwest::Client::new();
    let res = client.get(format!("{}/api/tags", url)).send().await?;
    let tags_response: TagsResponse = res.json().await?;
    Ok(tags_response.models)
}


pub async fn chat_stream(
    messages: Vec<String>,
    model: String,
    url: String,
    system_message: Option<String>,
) -> Result<LLMResponse, reqwest::Error> {
    let client = reqwest::Client::new();
    
    let mut ollama_messages = Vec::new();

    if let Some(sys_msg) = system_message {
        ollama_messages.push(serde_json::json!({"role": "system", "content": sys_msg}));
    }

    for msg in messages {
        if msg.starts_with("You: ") {
            ollama_messages.push(serde_json::json!({"role": "user", "content": msg.strip_prefix("You: ").unwrap()}));
        } else if msg.starts_with("Lucius: ") {
            ollama_messages.push(serde_json::json!({"role": "assistant", "content": msg.strip_prefix("Lucius: ").unwrap()}));
        } else if msg.starts_with("Tool Result: ") {
            ollama_messages.push(serde_json::json!({"role": "tool", "content": msg.strip_prefix("Tool Result: ").unwrap()}));
        } else if msg.starts_with("Tool Call: ") {
            ollama_messages.push(serde_json::json!({"role": "assistant", "content": msg}));
        }
    }
    
    let req_body = serde_json::json!({
        "model": model,
        "stream": true,
        "messages": ollama_messages,
    });
    
    let mut res = client
        .post(format!("{}/api/chat", url))
        .json(&req_body)
        .send()
        .await?;

    let mut full_response = String::new();
    while let Ok(Some(chunk)) = res.chunk().await {
        let text = String::from_utf8_lossy(&chunk);
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(chat_res) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(message) = chat_res["message"].as_object() {
                    if let Some(content) = message["content"].as_str() {
                        full_response.push_str(content);
                        if let Some(tool_call) = parse_tool_call(&full_response) {
                            return Ok(LLMResponse::ToolCallDetected(tool_call));
                        }
                    }
                }
                if chat_res["done"].as_bool().unwrap_or(false) {
                    return Ok(LLMResponse::FinalResponse(full_response));
                }
            } else {
                log::error!("Failed to parse stream chunk from /api/chat: {}", line);
            }
        }
    }
    if let Some(tool_call) = parse_tool_call(&full_response) {
        Ok(LLMResponse::ToolCallDetected(tool_call))
    } else {
        Ok(LLMResponse::FinalResponse(full_response))
    }
}
