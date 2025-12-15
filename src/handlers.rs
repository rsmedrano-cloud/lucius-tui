use std::time::Instant;
use crossterm::event::{Event, KeyCode, KeyModifiers, MouseEventKind};
use tui_textarea::{Input, TextArea};
use ratatui::widgets::{Block, Borders};
use crate::app::{App, SharedState};
use crate::ui::{AppMode, Focus, ConfirmationModal, Action};
// use crate::clipboard;
use crate::mouse;

pub async fn handle_event(app: &mut App<'_>, state: &mut SharedState, event: Event, should_quit: &mut bool) {
    log::info!("Handling event: {:?}", event);
    
    if let AppMode::Confirmation(ConfirmationModal::ExecuteTool { tool_call: _, confirm_tx }) = &mut state.mode {
        if let Event::Key(key) = event {
            if key.kind == crossterm::event::KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        if let Some(tx) = confirm_tx.take() {
                            let _ = tx.send(true);
                        }
                        state.mode = AppMode::Chat; // Exit modal
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        if let Some(tx) = confirm_tx.take() {
                            let _ = tx.send(false);
                        }
                        state.mode = AppMode::Chat; // Exit modal
                    }
                    _ => {}
                }
            }
        }
        return; // Don't process other events while in confirmation mode
    }

    match event {
        Event::Key(key) => {
            log::info!("Key event: {:?}", key);
            if key.kind == crossterm::event::KeyEventKind::Press {
                if key.modifiers == KeyModifiers::CONTROL {
                    match key.code {
                        KeyCode::Char('h') => {
                            state.mode = match state.mode {
                                AppMode::Help => AppMode::Chat,
                                _ => AppMode::Help,
                            };
                        }
                        KeyCode::Char('q') => *should_quit = true,
                        KeyCode::Char('s') => {
                            state.mode = AppMode::Settings;
                            let _ = app.action_tx.try_send(Action::RefreshModelsAndStatus);
                        }
                        KeyCode::Char('l') => {
                            state.chat_history.clear();
                            app.scroll = 0;
                        }
                        KeyCode::Char('c') | KeyCode::Char('y') => {
                            // if app.selection_range.is_none() {
                            //     if let Some(last_response) = state.chat_history.iter().rev().find(|m| m.starts_with("Lucius:")) {
                            //         let content_to_copy = last_response.strip_prefix("Lucius: ").unwrap_or(last_response).trim();
                            //         clipboard::copy_to_clipboard(content_to_copy.to_string()).await;
                            //         state.status_message = Some(("Copied last response to clipboard!".to_string(), Instant::now()));
                            //     } else {
                            //         log::warn!("Ctrl+C pressed, but no previous response from Lucius found to copy.");
                            //     }
                            // }
                        }
                        KeyCode::Char('r') if matches!(state.mode, AppMode::Settings) => {
                            state.config.ollama_url = Some(app.url_editor.lines().join(""));
                            state.config.save();
                            let _ = app.action_tx.try_send(Action::RefreshModelsAndStatus);
                        }
                        KeyCode::Char('t') => {
                            state.status_message = if state.redis_conn.is_some() {
                                Some(("MCP is connected via Redis.".to_string(), Instant::now()))
                            } else {
                                Some(("MCP Redis client not connected.".to_string(), Instant::now()))
                            };
                        }
                        _ => {}
                    }
                } else {
                    match &mut state.mode {
                        AppMode::Chat => match key.code {
                            KeyCode::Enter => {
                                let input = app.textarea.lines().join("\n");
                                if !input.trim().is_empty() {
                                    state.chat_history.push(format!("You: {}", input));
                                    app.scroll = u16::MAX;
                                    let _ = app.action_tx.try_send(Action::SendMessage(input));

                                    let mut textarea = TextArea::default();
                                    textarea.set_placeholder_text("Ask me anything...");
                                    textarea.set_block(
                                        Block::default().borders(Borders::ALL).title("Input").border_type(ratatui::widgets::BorderType::Rounded),
                                    );
                                    app.textarea = textarea;
                                }
                            }
                            _ => { app.textarea.input(Input::from(key)); }
                        },
                        AppMode::Settings => match app.focus {
                            Focus::Url => match key.code {
                                KeyCode::Tab => {
                                    state.config.ollama_url = Some(app.url_editor.lines().join(""));
                                    state.config.save();
                                    app.focus = Focus::McpUrl;
                                }
                                KeyCode::Enter | KeyCode::Esc => {
                                    state.config.ollama_url = Some(app.url_editor.lines().join(""));
                                    state.config.save();
                                    state.mode = AppMode::Chat;
                                }
                                _ => { app.url_editor.input(Input::from(key)); }
                            },
                            Focus::McpUrl => match key.code {
                                KeyCode::Tab => {
                                    state.config.mcp_redis_host = Some(app.mcp_url_editor.lines().join(""));
                                    state.config.save();
                                    app.focus = Focus::Models;
                                }
                                KeyCode::Enter | KeyCode::Esc => {
                                    state.config.mcp_redis_host = Some(app.mcp_url_editor.lines().join(""));
                                    state.config.save();
                                    state.mode = AppMode::Chat;
                                }
                                _ => { app.mcp_url_editor.input(Input::from(key)); }
                            },
                            Focus::Models => match key.code {
                                KeyCode::Esc | KeyCode::Enter => {
                                    if let Some(selected_index) = app.model_list_state.selected() {
                                        state.config.selected_model = state.models.get(selected_index).map(|m| m.name.clone());
                                        state.config.save();
                                    }
                                    state.mode = AppMode::Chat;
                                }
                                KeyCode::Down => app.models_next(state.models.len()),
                                KeyCode::Up => app.models_previous(state.models.len()),
                                KeyCode::Tab => { app.focus = Focus::Url; }
                                _ => {}
                            },
                        },
                        AppMode::Help => {
                            if key.code == KeyCode::Esc {
                                state.mode = AppMode::Chat;
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
                MouseEventKind::Down(_) => {
                    let (x, y) = (mouse_event.column, mouse_event.row);
                    if let Some(coords) = mouse::get_text_coordinates(app.conversation_area, x, y) {
                        app.selection_range = Some((coords, coords));
                    }
                }
                MouseEventKind::Drag(_) => {
                    if let Some((start, _)) = app.selection_range {
                        let (x, y) = (mouse_event.column, mouse_event.row);
                        if let Some(end) = mouse::get_text_coordinates(app.conversation_area, x, y) {
                            app.selection_range = Some((start, end));
                        }
                    }
                }
                MouseEventKind::Up(_) => {
                    // if let Some(((start_line, _), _)) = app.selection_range {
                    //     // Reconstruct the rendered text to find the clicked line.
                    //     // This is a temporary fix for the broken selection logic.
                    //     let history_text: String = state.chat_history.join("\n");
                    //     let markdown_text = termimad::MadSkin::default().term_text(&history_text).to_string();
                    //     let rendered_lines: Vec<&str> = markdown_text.lines().collect();

                    //     // The start_line is the screen line index.
                    //     if let Some(line_to_copy) = rendered_lines.get(start_line) {
                    //         clipboard::copy_to_clipboard(line_to_copy.to_string()).await;
                    //         state.status_message = Some(("Copied line to clipboard!".to_string(), Instant::now()));
                    //     }
                    // }
                    app.selection_range = None;
                }
                _ => {}
            }
        },
        _ => {}
    }
}
