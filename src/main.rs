use std::io::{self, stdout};
use std::sync::Arc;
use std::time::Duration;
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
use tokio::sync::{mpsc, Mutex};

mod app;
mod context;
mod config;
mod ui;
mod handlers;
mod renderer;
mod llm;
mod mouse;
mod clipboard;

use app::{App, SharedState};

use ui::Action;

use llm::{ping_ollama, fetch_models, chat_stream, LLMResponse};

use lucius::mcp;



async fn background_worker(

    state: Arc<Mutex<SharedState>>,

    mut action_rx: mpsc::Receiver<Action>,

) {

    loop {

        tokio::select! {

            Some(action) = action_rx.recv() => {

                let mut state_lock = state.lock().await;

                match action {

                    Action::RefreshModelsAndStatus => {

                        let url = state_lock.config.ollama_url.clone().unwrap_or_default();

                        state_lock.status = ping_ollama(url.clone()).await;

                        let msg = if state_lock.status { "Ollama is online." } else { "Ollama is offline." };

                        state_lock.status_message = Some((msg.to_string(), std::time::Instant::now()));



                        if state_lock.status {

                            if let Ok(models) = fetch_models(url).await {

                                state_lock.models = models;

                                state_lock.status_message = Some(("Models updated.".to_string(), std::time::Instant::now()));

                            }

                        } else {

                            state_lock.models = vec![];

                        }

                    }

                    Action::SendMessage(input) => {

                        let history = state_lock.chat_history.clone();

                        let model = state_lock.config.selected_model.clone().unwrap_or_default();

                        let url = state_lock.config.ollama_url.clone().unwrap_or_default();

                        let context = state_lock.lucius_context.clone();

                        

                        // Drop the lock so the UI can update while the LLM is thinking

                        drop(state_lock);



                        // This part needs its own state management for multi-turn tool use

                        let mut messages_for_llm = history;

                        messages_for_llm.push(format!("You: {}", input));



                        // The actual stream handling needs to be done here

                        match chat_stream(messages_for_llm, model, url, context).await {

                            Ok(llm_response) => {

                                let mut state_lock = state.lock().await;

                                match llm_response {

                                    LLMResponse::FinalResponse(text) => {

                                        state_lock.chat_history.push(format!("Lucius: {}", text));

                                    },

                                    LLMResponse::ToolCallDetected(tool) => {

                                        let tool_text = serde_json::to_string_pretty(&tool).unwrap_or_default();

                                        state_lock.chat_history.push(format!("Tool Call: {}", tool_text));



                                        if let Some(ref mut redis_conn) = state_lock.redis_conn {

                                            match mcp::submit_task(redis_conn, &tool).await {

                                                Ok(task_id) => {

                                                    match mcp::poll_result(redis_conn, &task_id).await {

                                                        Ok(result) => {

                                                            state_lock.chat_history.push(format!("Tool Result: {}", result));

                                                            

                                                            // TODO: Send the result back to the LLM for a final response.

                                                            // For now, just display the raw result.

                                                        },

                                                        Err(e) => {

                                                            state_lock.chat_history.push(format!("Error polling result: {}", e));

                                                        }

                                                    }

                                                },

                                                Err(e) => {

                                                    state_lock.chat_history.push(format!("Error submitting task: {}", e));

                                                }

                                            }

                                        } else {

                                            state_lock.chat_history.push("Error: Not connected to MCP.".to_string());

                                        }

                                    }

                                }

                            }

                            Err(e) => {

                                let mut state_lock = state.lock().await;

                                state_lock.chat_history.push(format!("Error: {}", e));

                            }

                        }

                    }

                }

            }

        }

    }

}




#[tokio::main]
async fn main() -> io::Result<()> {
    // 1. Initialize Logger
    if let Ok(log_file) = File::create("lucius.log") {
        WriteLogger::init(LevelFilter::Info, simplelog::Config::default(), log_file).unwrap();
        log::info!("Lucius TUI application starting...");
    } else {
        eprintln!("Failed to create log file. Continuing without logging.");
    }

    // 2. Setup Terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    stdout().execute(event::EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    // 3. Load Config and Create Shared State
    log::info!("Loading configuration...");
    let config = config::Config::load();
    let state = Arc::new(Mutex::new(SharedState::new(config.clone()).await));
    log::info!("Shared state created.");

    // 4. Create channels for UI actions
    let (action_tx, action_rx) = mpsc::channel(100);

    // 5. Spawn background worker
    tokio::spawn(background_worker(state.clone(), action_rx));

    // 6. Initialize App
    log::info!("Initializing App state...");
    let mut app = App::new(action_tx.clone(), &config);
    log::info!("App state initialized.");
    
    // 7. Trigger initial model and status refresh
    if let Err(e) = action_tx.send(Action::RefreshModelsAndStatus).await {
        log::error!("Failed to send initial model and status refresh action: {}", e);
    }

    // 8. Main Event Loop
    let mut should_quit = false;
    while !should_quit {
        // Draw UI
        terminal.draw(|frame| {
            if let Ok(state_lock) = state.try_lock() {
                renderer::draw_ui(frame, &mut app, &state_lock);
            }
        })?;

        // Handle UI events
        if event::poll(Duration::from_millis(50))? {
            let event = event::read()?;
            let mut state_lock = state.lock().await;
            handlers::handle_event(&mut app, &mut state_lock, event, &mut should_quit).await;
        }
    }

    // 9. Restore Terminal
    log::info!("Lucius TUI application shutting down.");
    stdout().execute(LeaveAlternateScreen)?;
    stdout().execute(event::DisableMouseCapture)?;
    disable_raw_mode()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn simple_test() {
        assert_eq!(2 + 2, 4);
    }
}
