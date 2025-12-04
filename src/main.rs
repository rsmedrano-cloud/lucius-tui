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
use tui_textarea::TextArea;
use termimad::MadSkin;

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    prompt: String,
    stream: bool,
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
    scroll: u16,
    interrupt_tx: watch::Sender<bool>,
    interrupt_rx: watch::Receiver<bool>,
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
        App {
            mode: AppMode::Chat,
            models: StatefulList::new(models),
            textarea,
            chat_history: vec![],
            url_editor,
            focus: Focus::Models,
            chat_rx,
            chat_tx,
            scroll: 0,
            interrupt_tx,
            interrupt_rx,
        }
    }

    fn scroll_up(&mut self) {
        if self.scroll > 0 {
            self.scroll -= 1;
        }
    }

    fn scroll_down(&mut self) {
        self.scroll += 1;
    }
}

async fn chat_stream(
    prompt: String,
    model: String,
    url: String,
    tx: mpsc::Sender<String>,
    mut interrupt_rx: watch::Receiver<bool>,
) -> Result<(), reqwest::Error> {
    let client = reqwest::Client::new();
    let req = ChatRequest {
        model,
        prompt,
        stream: true, // Enable streaming
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
            } else if let Some(last) = app.chat_history.last_mut() {
                if last.starts_with("Lucius: ") {
                    last.push_str(&msg);
                } else {
                    app.chat_history.push(format!("Lucius: {}", msg));
                }
            } else {
                app.chat_history.push(format!("Lucius: {}", msg));
            }
        }

        terminal.draw(|frame| {
            let area = frame.size();
            match app.mode {
                AppMode::Chat => {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Min(0), Constraint::Length(3), Constraint::Length(1)])
                        .split(area);
                    
                    let history_text: String = app.chat_history.join("\n");
                    let markdown_text = MadSkin::default().term_text(&history_text).to_string();
                    let history = Paragraph::new(Text::raw(markdown_text))
                        .wrap(Wrap { trim: true })
                        .scroll((app.scroll, 0))
                        .block(Block::default().title("Conversation").borders(Borders::ALL));
                    frame.render_widget(history, chunks[0]);

                    let textarea_widget = app.textarea.widget();
                    frame.render_widget(textarea_widget, chunks[1]);
                    
                    let help = Paragraph::new("Press ctrl+s to go to settings, ctrl+q to quit, ctrl+u/d to scroll, esc to interrupt")
                        .style(Style::default().fg(Color::Yellow));
                    frame.render_widget(help, chunks[2]);
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

                    let url_widget = app.url_editor.widget();
                    frame.render_widget(url_widget, chunks[0]);

                    let status = Paragraph::new("Status: Connected")
                        .style(Style::default().fg(Color::Green))
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
            if let Event::Key(key) = event::read()? {
                if key.kind == crossterm::event::KeyEventKind::Press {
                    match app.mode {
                        AppMode::Chat => {
                            if key.modifiers == KeyModifiers::CONTROL {
                                match key.code {
                                    KeyCode::Char('s') => app.mode = AppMode::Settings,
                                    KeyCode::Char('q') => should_quit = true,
                                    KeyCode::Char('u') => app.scroll_up(),
                                    KeyCode::Char('d') => app.scroll_down(),
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
                                        let tx = app.chat_tx.clone();
                                        let interrupt_rx = app.interrupt_rx.clone();
                                        let _ = app.interrupt_tx.send(false);
                                        tokio::spawn(async move {
                                            if let Err(e) = chat_stream(input, model, url, tx.clone(), interrupt_rx).await {
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
                                        app.textarea.input(key);
                                    }
                                }
                            }
                        }
                        AppMode::Settings => match app.focus {
                            Focus::Url => match key.code {
                                KeyCode::Tab => app.focus = Focus::Models,
                                _ => {
                                    app.url_editor.input(key);
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
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}




