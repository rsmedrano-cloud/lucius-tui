use std::io::{self, stdout};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers, MouseEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{ // pokemon reference? 
    prelude::{CrosstermBackend, Style, Terminal, Color, Modifier},
    layout::{Constraint, Direction, Layout},
    text::Text,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap, ListState},
};
use serde::{Deserialize, Serialize};
use simplelog::{Config, LevelFilter, WriteLogger};
use std::fs::File;
use tokio::sync::{mpsc, watch};
use tui_textarea::{Input, TextArea};
use termimad::MadSkin;

mod context;

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

#[derive(Deserialize, Clone)]
struct Model {
    name: String,
}

#[derive(Deserialize)]
struct TagsResponse {
    models: Vec<Model>,
}

#[derive(Deserialize, Clone)]
struct ChatResponse {
    response: Option<String>,
    done: Option<bool>,
}

enum AppMode {
    Chat,
    Settings,
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
    chat_rx: mpsc::Receiver<String>,
    chat_tx: mpsc::Sender<String>,
    interrupt_tx: watch::Sender<bool>,
    interrupt_rx: watch::Receiver<bool>,
    status: bool,
    scroll: u16,
    lucius_context: Option<String>,
}

impl<'a> App<'a> {
    fn new(models: Vec<Model>) -> App<'a> {
        let mut textarea = TextArea::default();
        textarea.set_placeholder_text("Ask me anything...");
        textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("Input"),
        );
        let mut url_editor = TextArea::new(vec!["http://192.168.1.42:11434".to_string()]);
        url_editor.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("Ollama URL"),
        );
        let (chat_tx, chat_rx) = mpsc::channel(100);
        let (interrupt_tx, interrupt_rx) = watch::channel(false);
        let lucius_context = context::load_lucius_context(); // Load context
        if let Some(ctx) = &lucius_context {
            log::info!("Loaded LUCIUS.md context: {} bytes", ctx.len());
        } else {
            log::info!("No LUCIUS.md context found.");
        }

        App {
            mode: AppMode::Chat,
            models: StatefulList::new(models),
            textarea,
            chat_history: vec![],
            url_editor,
            focus: Focus::Models,
            chat_rx,
            chat_tx,
            interrupt_tx,
            interrupt_rx,
            status: false,
            scroll: 0,
            lucius_context,
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


async fn chat_stream(
    prompt: String,
    model: String,
    url: String,
    tx: mpsc::Sender<String>,
    mut interrupt_rx: watch::Receiver<bool>,
    system_message: Option<String>,
) -> Result<(), reqwest::Error> {
    let client = reqwest::Client::new();
    let req = ChatRequest {
        model,
        prompt,
        stream: true, // Enable streaming
        system: system_message,
    };
    let mut res = client
        .post(format!("{}/api/generate", url))
        .json(&req)
        .send()
        .await?;

    loop {
        tokio::select! {
            chunk = res.chunk() => {
                if let Ok(Some(chunk)) = chunk {
                    let text = String::from_utf8_lossy(&chunk);
                    for line in text.lines() {
                        if line.trim().is_empty() {
                            continue;
                        }
                        if let Ok(stream_res) = serde_json::from_str::<ChatResponse>(line) {
                            if let Some(response_part) = stream_res.response {
                                if tx.send(response_part).await.is_err() {
                                    log::error!("Failed to send response part to channel");
                                    return Ok(());
                                }
                            }
                            if let Some(true) = stream_res.done {
                                if tx.send("<END_OF_STREAM>".to_string()).await.is_err() {
                                    log::error!("Failed to send end of stream message to channel");
                                }
                                return Ok(()); // End of stream
                            }
                        } else {
                            log::error!("Failed to parse stream chunk: {}", line);
                            if tx.send(format!("Error parsing chunk: {}", line)).await.is_err() {
                                log::error!("Failed to send error message to channel");
                            }
                            return Ok(());
                        }
                    }
                } else {
                    break;
                }
            },
            _ = interrupt_rx.changed() => {
                if *interrupt_rx.borrow() {
                    log::info!("Interrupt signal received, stopping stream.");
                    return Ok(());
                }
            }
        }
    }
    if tx.send("<END_OF_STREAM>".to_string()).await.is_err() {
        log::error!("Failed to send end of stream message to channel");
    }
    Ok(())
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
    WriteLogger::init(LevelFilter::Info, Config::default(), File::create("lucius.log").unwrap()).unwrap();

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    stdout().execute(event::EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let models = fetch_models("http://192.168.1.42:11434".to_string()).await.unwrap_or_else(|_| vec![]);
    let mut app = App::new(models);
    app.models.state.select(Some(0));
    
    let mut should_quit = false;
    while !should_quit {
        if let Ok(msg) = app.chat_rx.try_recv() {
            if msg == "<END_OF_STREAM>" {
                if let Some(last) = app.chat_history.last_mut() {
                    if last.starts_with("Lucius: ") {
                        let content = last.split_off(8);
                        let words: Vec<&str> = content.split_whitespace().collect();
                        *last = format!("Lucius: {}", words.join(" "));
                    }
                }
            } else { // Message is not <END_OF_STREAM>
                if let Some(last) = app.chat_history.last_mut() {
                    if last.starts_with("Lucius: ") {
                        last.push_str(&msg);
                    } else {
                        // Last message was not from Lucius, or it was a full message.
                        // Start a new Lucius response.
                        app.chat_history.push(format!("Lucius: {}", msg));
                    }
                } else {
                    // Chat history is empty, start a new Lucius response.
                    app.chat_history.push(format!("Lucius: {}", msg));
                }
            }
            app.scroll = u16::MAX; // Auto-scroll to bottom
        }

        terminal.draw(|frame| {
            let area = frame.area();
            match app.mode {
                AppMode::Chat => {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Min(0), Constraint::Length(1), Constraint::Length(3), Constraint::Length(1)])
                        .split(area);
                    
                    let history_text: String = app.chat_history.join("\n");
                    let markdown_text = MadSkin::default().term_text(&history_text).to_string();
                    
                    let chat_area_height = chunks[0].height as usize;
                    let num_lines_in_history = markdown_text.lines().count();
                    
                    let max_scroll_offset = if num_lines_in_history > chat_area_height {
                        (num_lines_in_history - chat_area_height) as u16
                    } else {
                        0
                    };

                    // Clamp app.scroll to valid range
                    app.scroll = app.scroll.min(max_scroll_offset);
                    
                    let history = Paragraph::new(Text::raw(markdown_text))
                        .wrap(Wrap { trim: true })
                        .scroll((app.scroll, 0)) // Use app.scroll here
                        .block(Block::default().title("Conversation").borders(Borders::ALL));
                    frame.render_widget(history, chunks[0]);

                    // Status line
                    let lucius_md_count = if app.lucius_context.is_some() { 1 } else { 0 };
                    let mcp_server_count = 0; // Placeholder for now
                    let status_text = format!("using: {} LUCIUS.md | {} MCP server", lucius_md_count, mcp_server_count);
                    let status_line = Paragraph::new(status_text)
                        .style(Style::default().fg(Color::DarkGray));
                    frame.render_widget(status_line, chunks[1]); // Render in new chunk

                    frame.render_widget(&app.textarea, chunks[2]); // Shifted to chunks[2]
                    
                    let help = Paragraph::new("Press ctrl+s to go to settings, ctrl+q to quit, ctrl+l to clear chat, mouse scroll up/down to scroll, esc to interrupt")
                        .style(Style::default().fg(Color::Yellow));
                    frame.render_widget(help, chunks[3]); // Shifted to chunks[3]
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
                    
                    let items: Vec<ListItem> = app.models.items.iter().map(|i| ListItem::new(i.name.as_str())).collect();
                    let list = List::new(items)
                        .block(Block::default().title("Models").borders(Borders::ALL))
                        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                        .highlight_symbol(">> ");

                    frame.render_stateful_widget(list, chunks[2], &mut app.models.state);

                    let help = Paragraph::new("Use TAB to switch focus, c to go to chat, r to refresh models, Enter to select, q to quit")
                        .style(Style::default().fg(Color::Yellow));
                    frame.render_widget(help, chunks[3]);
                }
            }
        })?;

        if event::poll(std::time::Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == crossterm::event::KeyEventKind::Press {
                        match app.mode {
                            AppMode::Chat => {
                                if key.modifiers == KeyModifiers::CONTROL {
                                    match key.code {
                                        KeyCode::Char('s') => {
                                            app.mode = AppMode::Settings;
                                            let url = app.url_editor.lines().join("");
                                            app.status = ping_ollama(url).await;
                                        }
                                        KeyCode::Char('q') => should_quit = true,
                                        KeyCode::Char('l') => app.chat_history.clear(),
                                        _ => {}
                                    }
                                } else {
                                    match key.code {
                                        KeyCode::Esc => {
                                            let _ = app.interrupt_tx.send(true);
                                        }
                                        KeyCode::Enter => {
                                            let input = app.textarea.lines().join("\n");
                                            let model = app.models.items[app.models.state.selected().unwrap_or(0)].name.clone();
                                            let url = app.url_editor.lines().join("");
                                            app.chat_history.push(format!("You: {}", input));
                                            app.scroll = u16::MAX;
                                            let tx = app.chat_tx.clone();
                                            let interrupt_rx = app.interrupt_rx.clone();
                                            let _ = app.interrupt_tx.send(false);
                                            let lucius_context = app.lucius_context.clone();
                                            tokio::spawn(async move {
                                                if let Err(e) = chat_stream(input, model, url, tx.clone(), interrupt_rx, lucius_context).await {
                                                    log::error!("Error in chat_stream spawn: {}", e);
                                                    if tx.send(format!("Error: {}", e)).await.is_err() {
                                                        log::error!("Failed to send error message to channel");
                                                    }
                                                }
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
                                        _ => {
                                            app.textarea.input(Input::from(key));
                                        }
                                    }
                                }
                            }
                            AppMode::Settings => match app.focus {
                                Focus::Url => match key.code {
                                    KeyCode::Tab => app.focus = Focus::Models,
                                    _ => {
                                        app.url_editor.input(Input::from(key));
                                    }
                                },
                                Focus::Models => match key.code {
                                    KeyCode::Char('q') => should_quit = true,
                                    KeyCode::Char('c') => app.mode = AppMode::Chat,
                                    KeyCode::Down => app.models.next(),
                                    KeyCode::Up => app.models.previous(),
                                    KeyCode::Tab => app.focus = Focus::Url,
                                    KeyCode::Char('r') => {
                                        let url = app.url_editor.lines().join("");
                                        let models = fetch_models(url).await.unwrap_or_else(|_| vec![]);
                                        app.models.items = models;
                                    }
                                    KeyCode::Enter => {
                                        app.mode = AppMode::Chat;
                                    }
                                    _ => {}
                                },
                            },
                        }
                    }
                }
                Event::Mouse(mouse_event) => match mouse_event.kind {
                    MouseEventKind::ScrollUp => app.scroll_up(),
                    MouseEventKind::ScrollDown => app.scroll_down(),
                    _ => {}
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



