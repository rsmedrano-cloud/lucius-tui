use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Instant;
use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseEventKind};
use ratatui::widgets::{Block, Borders};
use tokio::sync::mpsc; // Added oneshot
use tui_textarea::{Input, TextArea};
use crate::app::{self, App, AppMode, Focus, LLMResponse, ping_ollama, chat_stream, ConfirmationModal};
use crate::mcp::{self}; // Added submit_task and poll_result


pub async fn handle_event(app: &mut App<'_>, event: Event, should_quit: &mut bool) {
    // Check if app.mode is Confirmation, handle its events then skip other processing
    if let AppMode::Confirmation(ConfirmationModal::ExecuteTool { tool_call: _, confirm_tx }) = &mut app.mode {
        if let Event::Key(key) = event {
            if key.kind == crossterm::event::KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        if let Some(tx) = confirm_tx.take() {
                            let _ = tx.send(true);
                        }
                        app.mode = AppMode::Chat; // Exit modal
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        if let Some(tx) = confirm_tx.take() {
                            let _ = tx.send(false);
                        }
                        app.mode = AppMode::Chat; // Exit modal
                    }
                    _ => {}
                }
            }
        }
        return; // Don't process other events while in confirmation mode
    }

    if event::poll(std::time::Duration::from_millis(50)).expect("Event polling failed") {
        match event {
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
                                *should_quit = true;
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
                                app.models.items = app::fetch_models(url).await.unwrap_or_else(|_| vec![]);
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
                        match &mut app.mode { // Mutable reference to app.mode
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
                            AppMode::Confirmation(_) => {}
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

pub async fn handle_llm_turn(
    mut redis_conn: Option<redis::aio::MultiplexedConnection>,
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
                        match mcp::submit_task(conn, &tool_call).await {
                            Ok(task_id) => {
                                match mcp::poll_result(conn, &task_id).await {
                                    Ok(result_str) => {
                                        log::info!("Tool Result: {}", result_str);
                                        messages_for_llm.push(format!("Tool Result: {}", result_str));
                                    }
                                    Err(e) => {
                                        let err_msg = format!("Tool Error: {}", e);
                                        log::error!("{}", err_msg);
                                        messages_for_llm.push(format!("Tool Result: {}", err_msg));
                                    }
                                }
                            }
                            Err(e) => {
                                let err_msg = format!("Tool Error: {}", e);
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
