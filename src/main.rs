use crossterm::{
    event,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::{CrosstermBackend, Terminal};
use simplelog::{LevelFilter, WriteLogger};
use std::fs::File;
use std::io::{self, stdout};

mod app;
mod clipboard;
mod config;
mod context;
mod handlers;
mod llm;
mod mcp;
mod mouse;
mod renderer;
mod ui;

use app::App;
use llm::{fetch_models, ping_ollama};
use renderer::render_frame;
use ui::AppMode;

#[tokio::main]
async fn main() -> io::Result<()> {
    WriteLogger::init(
        LevelFilter::Info,
        simplelog::Config::default(),
        File::create("lucius.log").unwrap(),
    )
    .unwrap();

    let config = config::Config::load();
    let initial_ollama_url = config
        .ollama_url
        .clone()
        .unwrap_or_else(|| "http://192.168.1.42:11434".to_string());

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    stdout().execute(event::EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let models = fetch_models(initial_ollama_url.clone())
        .await
        .unwrap_or_else(|_| vec![]);
    let mut app = App::new(models, config.clone());

    if let Some(selected_model_name) = &config.selected_model {
        if let Some(index) = app
            .models
            .items
            .iter()
            .position(|m| &m.name == selected_model_name)
        {
            app.models.state.select(Some(index));
        }
    }

    if app.config.ollama_url.is_none() || app.models.state.selected().is_none() {
        app.mode = AppMode::Settings;
        if app.config.ollama_url.is_none() {
            app.status = ping_ollama(initial_ollama_url).await;
        } else if let Some(url) = &app.config.ollama_url {
            app.status = ping_ollama(url.clone()).await;
        }
    }

    let mut should_quit = false;
    while !should_quit {
        if let Some((_, time)) = app.status_message {
            if time.elapsed().as_secs() >= 2 {
                app.status_message = None;
            }
        }

        if let Ok(response) = app.response_rx.try_recv() {
            app.chat_history.push(format!("Lucius: {}", response));
            app.scroll = u16::MAX;
        }

        terminal.draw(|frame| {
            render_frame(frame, &mut app);
        })?;

        if event::poll(std::time::Duration::from_millis(50))? {
            let event = event::read()?;
            handlers::handle_events(&mut app, event, &mut should_quit).await;
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    stdout().execute(event::DisableMouseCapture)?;
    Ok(())
}
