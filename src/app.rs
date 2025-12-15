use std::time::Instant;
use ratatui::layout::Rect;
use ratatui::widgets::{ListState, Block, Borders};
use tokio::sync::mpsc;
use tui_textarea::TextArea;
use redis::aio::MultiplexedConnection;

use crate::config::{self, Config};
use crate::context;
use crate::llm::Model;
use crate::ui::{AppMode, Focus, Action};

/// Data that can be safely shared between the UI and background threads.
pub struct SharedState {
    pub mode: AppMode,
    pub models: Vec<Model>, // The actual data
    pub chat_history: Vec<String>,
    pub status: bool,
    pub lucius_context: Option<String>,
    pub config: config::Config,
    pub status_message: Option<(String, Instant)>,
    pub redis_conn: Option<MultiplexedConnection>,
}

impl SharedState {
    pub async fn new(initial_config: config::Config) -> Self {
        let lucius_context = context::load_lucius_context();
        if let Some(ctx) = &lucius_context {
            log::info!("Loaded LUCIUS.md context: {} bytes", ctx.len());
        } else {
            log::info!("No LUCIUS.md context found.");
        }

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

        Self {
            mode: AppMode::Chat,
            models: vec![],
            chat_history: vec![],
            status: false,
            lucius_context,
            config: initial_config,
            status_message: Some(("Connecting to Ollama...".to_string(), Instant::now())),
            redis_conn,
        }
    }
}


/// The main application struct, holding UI-specific state.
pub struct App<'a> {
    // UI-specific state
    pub model_list_state: ListState, // The UI state for the list
    pub textarea: TextArea<'a>,
    pub url_editor: TextArea<'a>,
    pub focus: Focus,
    pub scroll: u16,
    pub selection_range: Option<((usize, usize), (usize, usize))>,
    pub conversation_area: Rect,
    // Action channel to the background worker
    pub action_tx: mpsc::Sender<Action>,
}

impl<'a> App<'a> {
    pub fn new(
        action_tx: mpsc::Sender<Action>,
        initial_config: &Config
    ) -> App<'a> {
        let mut textarea = TextArea::default();
        textarea.set_placeholder_text("Ask me anything...");
        textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("Input")
                .border_type(ratatui::widgets::BorderType::Rounded),
        );
        
        let url_editor_content = initial_config.ollama_url.clone().unwrap_or_default();
        let mut url_editor = TextArea::new(vec![url_editor_content]);
        url_editor.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("Ollama URL"),
        );

        App {
            model_list_state: ListState::default(),
            textarea,
            url_editor,
            focus: Focus::Url,
            scroll: 0,
            selection_range: None,
            conversation_area: Rect::default(),
            action_tx,
        }
    }
    
    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    // Helper methods for list navigation
    pub fn models_next(&mut self, model_count: usize) {
        let i = match self.model_list_state.selected() {
            Some(i) => {
                if model_count == 0 { 0 }
                else if i >= model_count - 1 { 0 }
                else { i + 1 }
            }
            None => 0,
        };
        self.model_list_state.select(Some(i));
    }

    pub fn models_previous(&mut self, model_count: usize) {
        let i = match self.model_list_state.selected() {
            Some(i) => {
                if model_count == 0 { 0 }
                else if i == 0 { model_count - 1 }
                else { i - 1 }
            }
            None => 0,
        };
        self.model_list_state.select(Some(i));
    }
}
