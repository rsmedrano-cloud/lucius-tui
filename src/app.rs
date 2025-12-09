use std::time::Instant;
use ratatui::layout::Rect;
use ratatui::widgets::{ListState, Block, Borders};
use tokio::sync::mpsc;
use tui_textarea::TextArea;
use redis::aio::MultiplexedConnection;

use crate::config;
use crate::context;
use crate::llm::Model;
use crate::ui::{AppMode, Focus};

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
    pub selection_range: Option<((usize, usize), (usize, usize))>,
    pub conversation_area: Rect,
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
            focus: Focus::Url,
            response_rx,
            response_tx,
            status: false,
            scroll: 0,
            lucius_context,
            config: initial_config,
            status_message: None,
            redis_conn,
            selection_range: None,
            conversation_area: Rect::default(),
        }
    }
    
    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }
}