use ratatui::widgets::Block;
use ratatui::widgets::BorderType;
use ratatui::widgets::Borders;
use std::time::Instant;
use tokio::sync::mpsc;
use tui_textarea::TextArea;

use crate::config;
use crate::context;
use crate::llm::Model;
use crate::mcp;
use crate::ui::{AppMode, Focus, StatefulList};

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
    pub mcp_request_tx: Option<mpsc::Sender<mcp::McpRequest>>,
    pub conversation_rect: Option<ratatui::layout::Rect>,
    pub selection: Option<((usize, usize), (usize, usize))>,
    pub selecting: bool,
    pub display_lines: Vec<(usize, usize, String)>,
}

impl<'a> App<'a> {
    pub fn new(models: Vec<Model>, initial_config: config::Config) -> App<'a> {
        let mut textarea = TextArea::default();
        textarea.set_placeholder_text("Ask me anything...");
        textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("Input")
                .border_type(BorderType::Rounded),
        );

        let url_editor_content = initial_config
            .ollama_url
            .clone()
            .unwrap_or_else(|| "http://192.168.1.42:11434".to_string());
        let mut url_editor = TextArea::new(vec![url_editor_content]);
        url_editor.set_block(Block::default().borders(Borders::ALL).title("Ollama URL"));

        let (response_tx, response_rx) = mpsc::channel(100);
        let lucius_context = context::load_lucius_context();
        if let Some(ctx) = &lucius_context {
            log::info!("Loaded LUCIUS.md context: {} bytes", ctx.len());
        } else {
            log::info!("No LUCIUS.md context found.");
        }

        let mcp_server_name = "target/debug/shell-mcp";
        let mcp_request_tx = match mcp::McpClient::new(mcp_server_name) {
            Ok(client) => {
                log::info!("Successfully spawned '{}' server.", mcp_server_name);
                let (request_tx, request_rx) = mpsc::channel(10);
                tokio::spawn(mcp::mcp_manager_task(client, request_rx));
                Some(request_tx)
            }
            Err(e) => {
                log::warn!(
                    "Could not spawn '{}' server: {}. MCP functionality will be disabled.",
                    mcp_server_name,
                    e
                );
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
            mcp_request_tx,
            conversation_rect: None,
            selection: None,
            selecting: false,
            display_lines: vec![],
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }
}
