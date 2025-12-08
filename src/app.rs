use std::time::Instant;
use ratatui::{
    widgets::{ListState, Block, Borders, Padding},
    style::{Color, Style, Modifier},
};
use tokio::sync::mpsc;
use tui_textarea::TextArea;
use serde::Deserialize;
use redis::aio::MultiplexedConnection;
use uuid::Uuid;

use crate::config;
use crate::context;
use crate::mcp::{self, parse_tool_call, ToolCall, Task, TaskType}; // Updated path for mcp items

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

#[derive(PartialEq)] // Added for comparison in ConfirmationModal
pub enum AppMode {
    Chat,
    Settings,
    Help,
    Confirmation(ConfirmationModal), // New mode for confirmation dialog
}

#[derive(PartialEq)]
pub enum Focus {
    Url,
    Models,
}

#[derive(PartialEq)]
pub enum ConfirmationModal {
    ExecuteTool {
        tool_call: ToolCall,
        // The sender to send the confirmation back to the event loop
        confirm_tx: tokio::sync::oneshot::Sender<bool>, 
    },
}

pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T> StatefulList<T> {
    pub fn new(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

pub struct App<'a> {
    pub mode: AppMode,
    pub models: StatefulList<Model>,
    pub textarea: TextArea<'a>,
    pub chat_history: Vec<String>,
    pub url_editor: TextArea<'a>,
    pub focus: Focus,
    pub response_rx: mpsc::Receiver<String>,
    pub response_tx: mpsc::Sender<String>,
    pub status: bool,
    pub scroll: u16,
    pub lucius_context: Option<String>,
    pub config: config::Config,
    pub status_message: Option<(String, Instant)>,
    pub redis_conn: Option<MultiplexedConnection>,
}

impl<'a> App<'a> {
    pub async fn new(models: Vec<Model>, initial_config: config::Config) -> App<'a> {
        let mut textarea = TextArea::default();
        textarea.set_placeholder_text("Ask me anything...");
        textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("Input")
                .border_type(ratatui::widgets::BorderType::Rounded),
        );
        let url_editor_content = initial_config.ollama_url.clone().unwrap_or_else(|| "http://192.168.1.42:11434".to_string());
        let mut url_editor = TextArea::new(vec![url_editor_content]);
        url_editor.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("Ollama URL"),
        );
        let (response_tx, response_rx) = mpsc::channel(100);
        let lucius_context = context::load_lucius_context();
        if let Some(ctx) = &lucius_context {
            log::info!("Loaded LUCIUS.md context: {} bytes", ctx.len());
        } else {
            log::info!("No LUCIUS.md context found.");
        }

        // Initialize Redis connection for MCP
        let redis_host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let redis_url = format!("redis://{}/", redis_host);
        let redis_conn = match redis::Client::open(redis_url) {
            Ok(client) => match client.get_multiplexed_async_connection().await {
                Ok(conn) => {
                    log::info!("Successfully connected to Redis for MCP.");
                    Some(conn)
                },
                Err(e) => {
                    log::warn!("Failed to get multiplexed Redis connection: {}. MCP functionality will be disabled.", e);
                    None
                }
            },
            Err(e) => {
                log::warn!("Failed to create Redis client: {}. MCP functionality will be disabled.", e);
                None
            }
        };

        App {
            mode: AppMode::Chat,
            models: StatefulList::new(models),
            textarea,
            chat_history: vec![],
            url_editor,
            focus: Focus::Models,
            response_rx,
            response_tx,
            status: false,
            scroll: 0,
            lucius_context,
            config: initial_config,
            status_message: None,
            redis_conn,
        }
    }
    
    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }
}

pub async fn ping_ollama(url: String) -> bool {
    let client = reqwest::Client::new();
    let res = client.get(url).send().await;
    res.is_ok()
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
