use std::io::{self, stdout, Write};
use std::process::{Command, Stdio};
use std::time::Instant;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers, MouseEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    prelude::{CrosstermBackend, Style, Terminal, Color, Modifier, Layout, Direction, Constraint},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap, ListState, Padding},
    text::Text,
};
use serde::Deserialize;
use serde_json::Value;
use simplelog::{LevelFilter, WriteLogger};
use std::fs::File;

// Redis related imports
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;

// Tokio related imports
use tokio::sync::mpsc;

// UI related imports
use tui_textarea::{Input, TextArea};
use termimad::MadSkin;

// UUID import
use uuid::Uuid;

mod mcp;
mod context;
mod config;

// Specific mcp imports
use mcp::{parse_tool_call, ToolCall, Task, TaskType};

const HELP_MESSAGE: &str = r#"
--- Help ---
Ctrl+H: Toggle Help
Ctrl+S: Toggle Settings
Ctrl+Q: Quit
Ctrl+L: Clear Chat
Ctrl+Y: Yank (Copy) Last Response
Ctrl+T: MCP Status
Esc: Interrupt current stream (if any)
Mouse Scroll: Scroll chat history
Shift + Mouse Drag: Select text for copying
Enter: Send message (Chat mode), Select model (Settings mode)
Tab: Switch focus (Settings mode)
Ctrl+R: Refresh models (Settings mode)
Esc: Go to Chat (Settings mode)
-----------------
"#;

const ASCII_ART: &str = r#"
 _               _              ____ _     ___ 
| |   _   _  ___(_)_   _ ___   / ___| |   |_ _|
| |  | | | |/ __| | | | / __| | |   | |    | | 
| |__| |_| | (__| | |_| \__ \ | |___| |___ | | 
|_____\__,_|\___|_|\__,_|___/  \____|_____|___|
"#;

#[derive(Deserialize, Clone)]
struct Model {
    name: String,
}

#[derive(Deserialize)]
struct TagsResponse {
    models: Vec<Model>,
}

enum LLMResponse {
    FinalResponse(String),
    ToolCallDetected(ToolCall),
}

enum AppMode {
    Chat,
    Settings,
    Help,
}

enum Focus {
    Url,
    Models,
}

struct StatefulList<T> {
    state: ListState,
    items: Vec<T>,
}

impl<T> StatefulList<T> {
    fn new(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    fn next(&mut self) {
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

    fn previous(&mut self) {
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

struct App<'a> {
    mode: AppMode,
    models: StatefulList<Model>,
    textarea: TextArea<'a>,
    chat_history: Vec<String>,
    url_editor: TextArea<'a>,
    focus: Focus,
    response_rx: mpsc::Receiver<String>,
    response_tx: mpsc::Sender<String>,
    status: bool,
    scroll: u16,
    lucius_context: Option<String>,
    pub config: config::Config,
    status_message: Option<(String, Instant)>,
    redis_conn: Option<MultiplexedConnection>,
}

impl<'a> App<'a> {
    async fn new(models: Vec<Model>, initial_config: config::Config) -> App<'a> {
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
    
        fn scroll_up(&mut self) {
            self.scroll = self.scroll.saturating_sub(1);
        }
    
        fn scroll_down(&mut self) {
            self.scroll = self.scroll.saturating_add(1);
        }
    }
    
    async fn ping_ollama(url: String) -> bool {
        let client = reqwest::Client::new();
        let res = client.get(url).send().await;
        res.is_ok()
    }
    
async fn handle_llm_turn(
    mut redis_conn: Option<MultiplexedConnection>,
    current_history: Vec<String>,
    model: String,
    url: String,
    lucius_context: Option<String>,
    response_tx: mpsc::Sender<String>,
) {
    let mut messages_for_llm = current_history.clone();

    loop {
        match chat_stream(
            messages_for_llm.clone(),
            model.clone(),
            url.clone(),
            lucius_context.clone(),
        )
        .await
        {
            Ok(llm_response) => match llm_response {
                LLMResponse::FinalResponse(response_text) => {
                    if let Err(e) = response_tx.send(response_text).await {
                        log::error!("Failed to send final LLM response to main thread: {}", e);
                    }
                    break; 
                }
                LLMResponse::ToolCallDetected(tool_call) => {
                    log::info!("Tool Call Detected: {:?}", tool_call);
                    messages_for_llm.push(format!("Tool Call: {}", serde_json::to_string(&tool_call).unwrap_or_default()));

                    if let Some(conn) = &mut redis_conn {
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
                            details: tool_call.params,
                        };

                        let task_json = match serde_json::to_string(&task) {
                            Ok(json) => json,
                            Err(e) => {
                                let err_msg = format!("Tool Error: Failed to serialize task: {}", e);
                                log::error!("{}", err_msg);
                                messages_for_llm.push(format!("Tool Result: {}", err_msg));
                                continue;
                            }
                        };

                        let queue_key = "mcp::tasks::all";
                        let result_key = format!("mcp::result::{}", task_id);

                        // Push task to queue
                        let rpush_result: redis::RedisResult<()> = conn.rpush(queue_key, &task_json).await;
                        if let Err(e) = rpush_result {
                            let err_msg = format!("Tool Error: Failed to push task to Redis: {}", e);
                            log::error!("{}", err_msg);
                            messages_for_llm.push(format!("Tool Result: {}", err_msg));
                            continue;
                        }
                        log::info!("Pushed task {} to Redis queue '{}'", task_id, queue_key);

                        // Wait for result
                        log::info!("Waiting for result on key '{}'", result_key);
                        let blpop_result: redis::RedisResult<Vec<String>> = conn.blpop(&result_key, 30.0).await; // 30 second timeout

                        match blpop_result {
                            Ok(result_vec) => {
                                if let Some(result_str) = result_vec.get(1) {
                                    log::info!("Tool Result: {}", result_str);
                                    messages_for_llm.push(format!("Tool Result: {}", result_str));
                                } else {
                                    let err_msg = "Tool Error: Received empty result from Redis.";
                                    log::error!("{}", err_msg);
                                    messages_for_llm.push(format!("Tool Result: {}", err_msg));
                                }
                            }
                            Err(e) => {
                                let err_msg = format!("Tool Error: Failed to get result from Redis: {}", e);
                                log::error!("{}", err_msg);
                                messages_for_llm.push(format!("Tool Result: {}", err_msg));
                            }
                        }
                    } else {
                        let no_mcp_msg = "Tool Call detected, but MCP Redis client is not connected.";
                        log::error!("{}", no_mcp_msg);
                        messages_for_llm.push(format!("Tool Result: {}", no_mcp_msg));
                    }
                }
            },
            Err(e) => {
                let err_msg = format!("Error from chat stream: {}", e);
                log::error!("{}", err_msg);
                if let Err(send_err) = response_tx.send(err_msg).await {
                    log::error!("Failed to send chat stream error to main thread: {}", send_err);
                }
                break;
            }
        }
    }
}
    async fn chat_stream(
    messages: Vec<String>, // New argument for conversation history
    model: String,
    url: String,
    system_message: Option<String>,
) -> Result<LLMResponse, reqwest::Error> {
    let client = reqwest::Client::new();
    
    // Construct the messages for the Ollama API
    let mut ollama_messages = Vec::new();

    // Prepend the system message if it exists
    if let Some(sys_msg) = system_message {
        ollama_messages.push(serde_json::json!({"role": "system", "content": sys_msg}));
    }

    // Add previous messages from the conversation history
    for msg in messages {
        if msg.starts_with("You: ") {
            ollama_messages.push(serde_json::json!({"role": "user", "content": msg.strip_prefix("You: ").unwrap()}));
        } else if msg.starts_with("Lucius: ") {
            ollama_messages.push(serde_json::json!({"role": "assistant", "content": msg.strip_prefix("Lucius: ").unwrap()}));
        } else if msg.starts_with("Tool Result: ") {
            ollama_messages.push(serde_json::json!({"role": "tool", "content": msg.strip_prefix("Tool Result: ").unwrap()}));
        } else if msg.starts_with("Tool Call: ") {
            // Tool calls are made by the assistant, so they are part of the assistant's response.
            // The LLM will use a specific format for tool calls, e.g., "[TOOL_CALL] {...} [END_TOOL_CALL]"
            // We pass the full message so the LLM knows what it previously said.
            ollama_messages.push(serde_json::json!({"role": "assistant", "content": msg}));
        }
    }
    
    // Construct the request body with the messages
    let req_body = serde_json::json!({
        "model": model,
        "stream": true,
        "messages": ollama_messages,
    });
    
    let mut res = client
        .post(format!("{}/api/chat", url)) // Use /api/chat for multi-turn conversation
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
            // Ollama /api/chat endpoint returns a different structure
            if let Ok(chat_res) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(message) = chat_res["message"].as_object() {
                    if let Some(content) = message["content"].as_str() {
                        full_response.push_str(content);
                        // Check for tool call immediately as chunks are received
                        if let Some(tool_call) = parse_tool_call(&full_response) {
                            return Ok(LLMResponse::ToolCallDetected(tool_call));
                        }
                    }
                }
                if chat_res["done"].as_bool().unwrap_or(false) {
                    // If done: true is received and no tool call was detected mid-stream
                    return Ok(LLMResponse::FinalResponse(full_response));
                }
            } else {
                log::error!("Failed to parse stream chunk from /api/chat: {}", line);
            }
        }
    }
    // If stream ends without done: true, check for tool call one last time
    if let Some(tool_call) = parse_tool_call(&full_response) {
        Ok(LLMResponse::ToolCallDetected(tool_call))
    } else {
        Ok(LLMResponse::FinalResponse(full_response))
    }
}
    async fn fetch_models(url: String) -> Result<Vec<Model>, reqwest::Error> {
        let client = reqwest::Client::new();
        let res = client
            .get(format!("{}/api/tags", url))
            .send()
            .await?;
    
        let res_json: TagsResponse = res.json().await?;
        Ok(res_json.models)
    }
    
    #[tokio::main]
    async fn main() -> io::Result<()> {
        WriteLogger::init(LevelFilter::Info, simplelog::Config::default(), File::create("lucius.log").unwrap()).unwrap();
    
        let config = config::Config::load(); // Load config
        let initial_ollama_url = config.ollama_url.clone().unwrap_or_else(|| "http://192.168.1.42:11434".to_string());
    
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        stdout().execute(event::EnableMouseCapture)?;
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    
        let models = fetch_models(initial_ollama_url.clone()).await.unwrap_or_else(|_| vec![]);
        let mut app = App::new(models, config.clone()).await; // App::new is now async
    
        // Select initial model if present in config
        if let Some(selected_model_name) = &config.selected_model { // Use config here directly
            if let Some(index) = app.models.items.iter().position(|m| &m.name == selected_model_name) {
                app.models.state.select(Some(index));
            }
        }
    
        // Initial setup flow: if no URL or model is selected, go to Settings mode
        if app.config.ollama_url.is_none() || app.models.state.selected().is_none() {
            app.mode = AppMode::Settings;
            // Also ping the default URL to check status, if no URL was configured
            if app.config.ollama_url.is_none() {
                app.status = ping_ollama(initial_ollama_url).await;
            } else if let Some(url) = &app.config.ollama_url { // Ping configured URL if present
                app.status = ping_ollama(url.clone()).await;
            }
        }
        
        let mut should_quit = false;
        while !should_quit {
            // Clear status message after a timeout
            if let Some((_, time)) = app.status_message {
                if time.elapsed().as_secs() >= 2 {
                    app.status_message = None;
                }
            }

            // Check for a new, complete response from the LLM
            if let Ok(response) = app.response_rx.try_recv() {
                // For now, just push the full response. Tool parsing will be added here.
                app.chat_history.push(format!("Lucius: {}", response));
                app.scroll = u16::MAX; // Auto-scroll to bottom
            }

            terminal.draw(|frame| {
                let area = frame.area();
                match app.mode {
                    AppMode::Chat => {
                        let chunks = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Length(7), // For ASCII Art
                                Constraint::Min(0),    // For Conversation
                                Constraint::Length(1), // For Status Line
                                Constraint::Length(3), // For Input
                                Constraint::Length(1), // For Bottom Bar
                            ])
                            .split(area);
    
                        // ASCII Art
                        let ascii_art = Paragraph::new(ASCII_ART)
                            .alignment(ratatui::layout::Alignment::Center);
                        frame.render_widget(ascii_art, chunks[0]);
                        
                        // Conversation History
                        let history_text: String = app.chat_history.join("\n");
                        let markdown_text = MadSkin::default().term_text(&history_text).to_string();

                        let conversation_block = Block::default()
                            .title("Conversation")
                            .borders(Borders::ALL)
                            .border_type(ratatui::widgets::BorderType::Rounded)
                            .padding(Padding::new(1, 1, 1, 1)); // Left, Right, Top, Bottom

                        let chat_area_height = chunks[1].height.saturating_sub(2) as usize;
                        let num_lines_in_history = markdown_text.lines().count();
                        
                        let max_scroll_offset = if num_lines_in_history > chat_area_height {
                            (num_lines_in_history - chat_area_height) as u16
                        } else {
                            0
                        };
    
                        app.scroll = app.scroll.min(max_scroll_offset);
                        
                        let history = Paragraph::new(Text::raw(markdown_text))
                            .wrap(Wrap { trim: true })
                            .scroll((app.scroll, 0))
                            .block(conversation_block);
                        frame.render_widget(history, chunks[1]);
    
                        // Status line
                        let status_text = if let Some((msg, _)) = &app.status_message {
                            msg.clone()
                        } else {
                            let lucius_md_count = if app.lucius_context.is_some() { 1 } else { 0 };
                            let mcp_server_count = if app.redis_conn.is_some() { 1 } else { 0 };
                            format!("using: {} LUCIUS.md | {} MCP server", lucius_md_count, mcp_server_count)
                        };
                        let status_line = Paragraph::new(status_text)
                            .style(if app.status_message.is_some() {
                                Style::default().fg(Color::Green)
                            } else {
                                Style::default().fg(Color::DarkGray)
                            });
                        frame.render_widget(status_line, chunks[2]); // Render in new chunk
    
                        frame.render_widget(&app.textarea, chunks[3]); // Shifted to chunks[3]
                        
                        // Display current directory and active model
                        let bottom_chunks = Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                            .split(chunks[4]); // Use the last chunk for this
    
                        let current_dir = std::env::current_dir()
                            .map(|path| path.display().to_string())
                            .unwrap_or_else(|_| "Error getting dir".to_string());
                        let dir_paragraph = Paragraph::new(format!("Dir: {}", current_dir))
                            .style(Style::default().fg(Color::Blue));
                        frame.render_widget(dir_paragraph, bottom_chunks[0]);
    
                        let active_model_name = app.models.items.get(app.models.state.selected().unwrap_or(0))
                            .map(|model| model.name.clone())
                            .unwrap_or_else(|| "No model selected".to_string());
                        let model_paragraph = Paragraph::new(format!("Model: {}", active_model_name))
                            .alignment(ratatui::layout::Alignment::Right)
                            .style(Style::default().fg(Color::LightCyan));
                        frame.render_widget(model_paragraph, bottom_chunks[1]);
                    }
                    AppMode::Settings => {
                        let chunks = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Length(3),
                                Constraint::Length(3),
                                Constraint::Min(0),
                                Constraint::Length(1),
                            ])
                            .split(area);
    
                        // URL Editor
                        let url_editor_block = Block::default()
                            .borders(Borders::ALL)
                            .title("Ollama URL");
                        if let Focus::Url = app.focus {
                            app.url_editor.set_block(url_editor_block.clone().border_style(Style::default().fg(Color::LightCyan)));
                        } else {
                            app.url_editor.set_block(url_editor_block);
                        }
                        frame.render_widget(&app.url_editor, chunks[0]);
    
                        let (status_text, status_color) = if app.status {
                            ("Status: Connected", Color::Green)
                        } else {
                            ("Status: Disconnected", Color::Red)
                        };
                        let status = Paragraph::new(status_text)
                            .style(Style::default().fg(status_color))
                            .block(Block::default().title("Status").borders(Borders::ALL));
                        frame.render_widget(status, chunks[1]);
                        
                        // Models List
                        let models_block = Block::default()
                            .title("Models")
                            .borders(Borders::ALL);
                        let items: Vec<ListItem> = app.models.items.iter().map(|i| ListItem::new(i.name.as_str())).collect();
                        let list = List::new(items)
                            .block(if let Focus::Models = app.focus {
                                models_block.border_style(Style::default().fg(Color::LightCyan))
                            } else {
                                models_block
                            })
                            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                            .highlight_symbol(">> ");
    
                        frame.render_stateful_widget(list, chunks[2], &mut app.models.state);
    
                        // Removed help paragraph from here
                    }
                    AppMode::Help => {
                        let help_block = Block::default()
                            .title("Help")
                            .borders(Borders::ALL);
                        let help_paragraph = Paragraph::new(HELP_MESSAGE)
                            .wrap(Wrap { trim: true })
                            .block(help_block);
                        frame.render_widget(help_paragraph, area);
                    }
                }
            })?;
    
            if event::poll(std::time::Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key) => {
                        if key.kind == crossterm::event::KeyEventKind::Press {
                            // Handle global shortcuts first
                            if key.modifiers == KeyModifiers::CONTROL {
                                match key.code {
                                    KeyCode::Char('h') => {
                                        app.mode = match app.mode {
                                            AppMode::Help => AppMode::Chat,
                                            _ => AppMode::Help,
                                        };
                                    }
                                    KeyCode::Char('q') => {
                                        should_quit = true;
                                    }
                                    KeyCode::Char('s') => {
                                        app.mode = AppMode::Settings;
                                        let url = app.config.ollama_url.clone().unwrap_or_default();
                                        app.status = ping_ollama(url).await;
                                    }
                                    KeyCode::Char('l') => {
                                        app.chat_history.clear();
                                        app.scroll = 0;
                                    }
                                    KeyCode::Char('y') => {
                                        log::info!("Ctrl+Y detected: Attempting to copy last response via wl-copy.");
                                        if let Some(last_response) = app.chat_history.iter().rev().find(|m| m.starts_with("Lucius:")) {
                                            log::info!("Found last response to copy.");
                                            let content_to_copy = last_response.strip_prefix("Lucius: ").unwrap_or(last_response).trim();
                                            log::info!("Attempting to copy content: \"{}\"", content_to_copy);
                                            
                                            let process = Command::new("wl-copy")
                                                .stdin(Stdio::piped())
                                                .spawn();

                                            if let Ok(mut child) = process {
                                                if let Some(mut stdin) = child.stdin.take() {
                                                    if stdin.write_all(content_to_copy.as_bytes()).is_ok() {
                                                        log::info!("Successfully wrote to wl-copy stdin.");
                                                        app.status_message = Some(("Copied last response to clipboard!".to_string(), Instant::now()));
                                                    } else {
                                                        log::error!("Failed to write to wl-copy stdin.");
                                                    }
                                                } else {
                                                    log::error!("Could not get stdin for wl-copy process.");
                                                }
                                            } else {
                                                log::error!("Failed to spawn wl-copy process.");
                                            }
                                        } else {
                                            log::warn!("Ctrl+Y pressed, but no previous response from Lucius found to copy.");
                                        }
                                    }
                                    KeyCode::Char('r') if matches!(app.mode, AppMode::Settings) => {
                                        app.config.ollama_url = Some(app.url_editor.lines().join(""));
                                        app.config.save();
                                        let url = app.config.ollama_url.clone().unwrap_or_default();
                                        app.models.items = fetch_models(url).await.unwrap_or_else(|_| vec![]);
                                        app.models.state.select(Some(0));
                                    }
                                    KeyCode::Char('t') => {
                                        if app.redis_conn.is_some() {
                                            app.status_message = Some(("MCP is connected via Redis.".to_string(), Instant::now()));
                                        } else {
                                            app.status_message = Some(("MCP Redis client not connected.".to_string(), Instant::now()));
                                        }
                                    }
                                    _ => {
                                        // If no global Ctrl shortcut matches, do nothing.
                                        // Mode-specific input will be handled by the outer `else` block if applicable.
                                    }
                                }
                            } else {
                                // Handle non-Ctrl keys based on mode
                                match app.mode {
                                    AppMode::Chat => match key.code {
                                        KeyCode::Enter => {
                                            let input = app.textarea.lines().join("\n");
                                            if !input.trim().is_empty() {
                                                let model = app.models.items.get(app.models.state.selected().unwrap_or(0))
                                                    .map(|model| model.name.clone())
                                                    .unwrap_or_else(|| "No model selected".to_string());
                                                let url = app.config.ollama_url.clone().unwrap_or_default();
                                                app.chat_history.push(format!("You: {}", input));
                                                app.scroll = u16::MAX;
                                                
                                                let response_tx_clone = app.response_tx.clone();
                                                let lucius_context_clone = app.lucius_context.clone();
                                                let chat_history_clone = app.chat_history.clone();
                                                let redis_conn_clone = app.redis_conn.clone();
                                                
                                                tokio::spawn(async move {
                                                    handle_llm_turn(
                                                        redis_conn_clone,
                                                        chat_history_clone,
                                                        model,
                                                        url,
                                                        lucius_context_clone,
                                                        response_tx_clone,
                                                    )
                                                    .await;
                                                });

                                                let mut textarea = TextArea::default();
                                                textarea.set_placeholder_text("Ask me anything...");
                                                textarea.set_block(
                                                    Block::default()
                                                        .borders(Borders::ALL)
                                                        .title("Input"),
                                                );
                                                app.textarea = textarea;
                                            }
                                        }
                                        _ => {
                                            app.textarea.input(Input::from(key));
                                        }
                                    },
                                    AppMode::Settings => match app.focus {
                                        Focus::Url => match key.code {
                                            KeyCode::Tab | KeyCode::Enter => {
                                                app.focus = Focus::Models;
                                                app.config.ollama_url = Some(app.url_editor.lines().join(""));
                                                app.config.save();
                                            }
                                            KeyCode::Esc => {
                                                app.mode = AppMode::Chat;
                                                app.config.ollama_url = Some(app.url_editor.lines().join(""));
                                                app.config.save();
                                            }
                                            _ => {
                                                app.url_editor.input(Input::from(key));
                                            }
                                        },
                                        Focus::Models => match key.code {
                                            KeyCode::Esc => {
                                                if let Some(selected_index) = app.models.state.selected() {
                                                    app.config.selected_model = app.models.items.get(selected_index).map(|m| m.name.clone());
                                                    app.config.save();
                                                }
                                                app.mode = AppMode::Chat;
                                            }
                                            KeyCode::Down => app.models.next(),
                                            KeyCode::Up => app.models.previous(),
                                            KeyCode::Tab => {
                                                app.focus = Focus::Url;
                                            }
                                            KeyCode::Enter => {
                                                if let Some(selected_index) = app.models.state.selected() {
                                                    app.config.selected_model = app.models.items.get(selected_index).map(|m| m.name.clone());
                                                    app.config.save();
                                                }
                                                app.mode = AppMode::Chat;
                                            }
                                            _ => {}
                                        },
                                    },
                                    AppMode::Help => {
                                        if key.code == KeyCode::Esc {
                                            app.mode = AppMode::Chat;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Event::Mouse(mouse_event) => {
                        match mouse_event.kind {
                            MouseEventKind::ScrollUp => app.scroll_up(),
                            MouseEventKind::ScrollDown => app.scroll_down(),
                            _ => {}
                        }
                    },
                    _ => {}
                }
            }
        }
    
        disable_raw_mode()?;
        stdout().execute(LeaveAlternateScreen)?;
        stdout().execute(event::DisableMouseCapture)?;
        Ok(())
    }
    

