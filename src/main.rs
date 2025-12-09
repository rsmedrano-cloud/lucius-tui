use std::io::{self, stdout};
use crossterm::{
    event::{self},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use simplelog::{LevelFilter, WriteLogger};
use std::fs::File;

// Import our new modules
mod app;
mod context;
mod config;
mod mcp; // mcp.rs will contain data structures and maybe task submission helper
mod ui;
mod events;

// Import items directly from app module
use app::{App, AppMode, ping_ollama};


// ASCII Art and HELP_MESSAGE should ideally be moved to ui.rs or a separate consts.rs
// For now, keeping them here as they are part of the initial "bare minimum" main.rs until ui.rs is created



#[tokio::main]
async fn main() -> io::Result<()> {
    // 1. Initialize Logger
    WriteLogger::init(LevelFilter::Info, simplelog::Config::default(), File::create("lucius.log").unwrap()).unwrap();
    log::info!("Lucius TUI application starting...");

    // 2. Setup Terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    stdout().execute(event::EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    // 3. Load Config and Initial Models
    let config = config::Config::load();
    let initial_ollama_url = config.ollama_url.clone().unwrap_or_else(|| "http://192.168.1.42:11434".to_string());
    let models = app::fetch_models(initial_ollama_url.clone()).await.unwrap_or_else(|e| {
        log::error!("Failed to fetch initial models: {}", e);
        vec![]
    });

    // 4. Initialize App
    let mut app = App::new(models, config.clone()).await;

    // 5. Initial Setup Flow (e.g., go to Settings if no URL/model selected)
    if app.config.ollama_url.is_none() || app.models.state.selected().is_none() {
        app.mode = AppMode::Settings;
        if app.config.ollama_url.is_none() {
            app.status = ping_ollama(initial_ollama_url).await;
        } else if let Some(url) = &app.config.ollama_url {
            app.status = ping_ollama(url.clone()).await;
        }
    }
    
    // 6. Main Event Loop
    let mut should_quit = false;
    while !should_quit {
        // Clear status message after a timeout
        if let Some((_, time)) = app.status_message {
            if time.elapsed().as_secs() >= 2 {
                app.status_message = None;
            }
        }

        // Handle incoming LLM responses
        if let Ok(response) = app.response_rx.try_recv() {
            app.chat_history.push(format!("Lucius: {}", response));
            app.scroll = u16::MAX; // Auto-scroll to bottom
        }

        // Draw UI
        terminal.draw(|frame| {
            ui::draw_ui(frame, &mut app);
        })?;

        // Handle events
        if let Ok(true) = event::poll(std::time::Duration::from_millis(50)) {
            if let Ok(event) = event::read() {
                events::handle_event(&mut app, event, &mut should_quit).await;
            }
        }
    }

    // 7. Restore Terminal
    log::info!("Lucius TUI application shutting down.");
    stdout().execute(LeaveAlternateScreen)?;
    stdout().execute(event::DisableMouseCapture)?;
    disable_raw_mode()?;
    Ok(())
}