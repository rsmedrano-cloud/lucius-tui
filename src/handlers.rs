use crate::app::App;
use crate::clipboard::{copy_to_clipboard, copy_to_primary};
use crate::llm::{fetch_models, handle_llm_turn, ping_ollama};
use crate::mcp;
use crate::mouse::{cursor_in_conversation, mouse_pos_to_selpos};
use crate::renderer::extract_selection_text;
use crate::ui::{AppMode, Focus};
use crossterm::event::{Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use serde_json::Value;
use std::time::Instant;
use tui_textarea::Input;
use tui_textarea::TextArea;

pub async fn handle_ctrl_y(app: &mut App<'_>) {
    if let Some(selection) = app.selection {
        let content_to_copy = extract_selection_text(&app.chat_history, selection);
        match copy_to_clipboard(&content_to_copy) {
            Ok(_) => {
                app.status_message =
                    Some(("Copied selection to clipboard!".to_string(), Instant::now()));
            }
            Err(e) => {
                app.status_message = Some((format!("Copy failed: {}", e), Instant::now()));
            }
        }
    } else if let Some(last_response) = app
        .chat_history
        .iter()
        .rev()
        .find(|m| m.starts_with("Lucius:"))
    {
        let content_to_copy = last_response
            .strip_prefix("Lucius: ")
            .unwrap_or(last_response)
            .trim();
        match copy_to_clipboard(content_to_copy) {
            Ok(_) => {
                app.status_message = Some((
                    "Copied last response to clipboard!".to_string(),
                    Instant::now(),
                ));
            }
            Err(e) => {
                app.status_message = Some((format!("Copy failed: {}", e), Instant::now()));
            }
        }
    } else {
        app.status_message = Some((
            "No selection or response to copy".to_string(),
            Instant::now(),
        ));
    }
}

pub async fn handle_ctrl_r(app: &mut App<'_>) {
    app.config.ollama_url = Some(app.url_editor.lines().join(""));
    app.config.save();
    let url = app.config.ollama_url.clone().unwrap_or_default();
    app.models.items = fetch_models(url).await.unwrap_or_else(|_| vec![]);
    app.models.state.select(Some(0));
}

pub fn handle_chat_enter(app: &mut App<'_>) {
    let input = app.textarea.lines().join("\n");
    if !input.trim().is_empty() {
        let model = app
            .models
            .items
            .get(app.models.state.selected().unwrap_or(0))
            .map(|model| model.name.clone())
            .unwrap_or_else(|| "No model selected".to_string());
        let url = app.config.ollama_url.clone().unwrap_or_default();
        app.chat_history.push(format!("You: {}", input));
        app.scroll = u16::MAX;

        let response_tx_clone = app.response_tx.clone();
        let lucius_context_clone = app.lucius_context.clone();
        let chat_history_clone = app.chat_history.clone();
        let mcp_request_tx_clone = app.mcp_request_tx.clone();

        tokio::spawn(async move {
            handle_llm_turn(
                mcp_request_tx_clone,
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
        textarea.set_block(Block::default().borders(Borders::ALL).title("Input"));
        app.textarea = textarea;
    }
}

pub fn handle_left_mouse_up(app: &mut App<'_>) {
    if app.selecting && app.selection.is_some() {
        if let Some(((s_msg, s_char), (e_msg, e_char))) = app.selection {
            let mut parts: Vec<String> = vec![];
            if s_msg == e_msg {
                if let Some(msg) = app.chat_history.get(s_msg) {
                    let chars: Vec<char> = msg.chars().collect();
                    let start = s_char.min(chars.len());
                    let end = e_char.min(chars.len().saturating_sub(1));
                    if start <= end && end < chars.len() {
                        parts.push(chars[start..=end].iter().collect());
                    }
                }
            } else {
                if let Some(msg) = app.chat_history.get(s_msg) {
                    let chars: Vec<char> = msg.chars().collect();
                    let start = s_char.min(chars.len());
                    if start < chars.len() {
                        parts.push(chars[start..].iter().collect());
                    }
                }
                for idx in (s_msg + 1)..e_msg {
                    if let Some(m) = app.chat_history.get(idx) {
                        parts.push(m.clone());
                    }
                }
                if let Some(msg) = app.chat_history.get(e_msg) {
                    let chars: Vec<char> = msg.chars().collect();
                    let end = e_char.min(chars.len().saturating_sub(1));
                    if end < chars.len() {
                        parts.push(chars[..=end].iter().collect());
                    }
                }
            }
            let content_to_copy = parts.join("\n");
            match copy_to_clipboard(&content_to_copy) {
                Ok(_) => {
                    app.status_message = Some(("Copied to clipboard!".to_string(), Instant::now()));
                }
                Err(e) => {
                    app.status_message = Some((format!("Copy failed: {}", e), Instant::now()));
                }
            }
        }
    }
    app.selecting = false;
}

pub fn handle_middle_mouse_up(app: &mut App<'_>) {
    if app.selecting && app.selection.is_some() {
        if let Some(((s_msg, s_char), (e_msg, e_char))) = app.selection {
            let mut parts: Vec<String> = vec![];
            if s_msg == e_msg {
                if let Some(msg) = app.chat_history.get(s_msg) {
                    let chars: Vec<char> = msg.chars().collect();
                    let start = s_char.min(chars.len());
                    let end = e_char.min(chars.len().saturating_sub(1));
                    if start <= end && end < chars.len() {
                        parts.push(chars[start..=end].iter().collect());
                    }
                }
            } else {
                if let Some(msg) = app.chat_history.get(s_msg) {
                    let chars: Vec<char> = msg.chars().collect();
                    let start = s_char.min(chars.len());
                    if start < chars.len() {
                        parts.push(chars[start..].iter().collect());
                    }
                }
                for idx in (s_msg + 1)..e_msg {
                    if let Some(m) = app.chat_history.get(idx) {
                        parts.push(m.clone());
                    }
                }
                if let Some(msg) = app.chat_history.get(e_msg) {
                    let chars: Vec<char> = msg.chars().collect();
                    let end = e_char.min(chars.len().saturating_sub(1));
                    if end < chars.len() {
                        parts.push(chars[..=end].iter().collect());
                    }
                }
            }
            let content_to_copy = parts.join("\n");
            let _ = copy_to_clipboard(&content_to_copy);
            let _ = copy_to_primary(&content_to_copy);
            app.status_message = Some((
                "Copied to clipboard and primary!".to_string(),
                Instant::now(),
            ));
        }
    }
    app.selecting = false;
}

pub async fn handle_ctrl_t(app: &mut App<'_>) {
    if let Some(mcp_tx) = &app.mcp_request_tx {
        let (oneshot_tx, oneshot_rx) = tokio::sync::oneshot::channel();
        let mcp_req = mcp::McpRequest {
            method: "list_tools".to_string(),
            params: Value::Null,
            response_tx: oneshot_tx,
        };

        match mcp_tx.send(mcp_req).await {
            Ok(_) => match oneshot_rx.await {
                Ok(result_or_err) => match result_or_err {
                    Ok(result) => {
                        app.status_message =
                            Some((format!("MCP Tools: {}", result), Instant::now()));
                    }
                    Err(e) => {
                        app.status_message = Some((format!("MCP Error: {}", e), Instant::now()));
                        log::error!("MCP Client call error: {}", e);
                    }
                },
                Err(e) => {
                    app.status_message = Some((
                        format!("MCP Error: Failed to receive response: {}", e),
                        Instant::now(),
                    ));
                    log::error!("MCP Client response channel error: {}", e);
                }
            },
            Err(e) => {
                app.status_message = Some((
                    format!("MCP Error: Failed to send request: {}", e),
                    Instant::now(),
                ));
                log::error!("MCP Client request channel error: {}", e);
            }
        }
    } else {
        app.status_message = Some(("MCP client not running.".to_string(), Instant::now()));
    }
}

// Mouse handlers

pub fn handle_mouse_scroll_up(app: &mut App<'_>, row: u16, column: u16) {
    if cursor_in_conversation(app.conversation_rect, row, column) {
        app.scroll_up();
    }
}

pub fn handle_mouse_scroll_down(app: &mut App<'_>, row: u16, column: u16) {
    if cursor_in_conversation(app.conversation_rect, row, column) {
        app.scroll_down();
    }
}

pub fn handle_left_mouse_down(app: &mut App<'_>, row: u16, column: u16) {
    if let Some((msg, ch)) = mouse_pos_to_selpos(
        app.conversation_rect,
        &app.display_lines,
        app.scroll,
        row,
        column,
    ) {
        app.selection = Some(((msg, ch), (msg, ch)));
        app.selecting = true;
        app.status_message = Some((
            "Selecting... (will copy on release)".to_string(),
            Instant::now(),
        ));
    }
}

pub fn handle_left_mouse_drag(app: &mut App<'_>, row: u16, column: u16) {
    if app.selecting {
        if let Some((msg, ch)) = mouse_pos_to_selpos(
            app.conversation_rect,
            &app.display_lines,
            app.scroll,
            row,
            column,
        ) {
            if let Some((start_pos, _)) = app.selection {
                let a = start_pos;
                let b = (msg, ch);
                let (s, e) = if a <= b { (a, b) } else { (b, a) };
                app.selection = Some((s, e));
            } else {
                app.selection = Some(((msg, ch), (msg, ch)));
            }
        }
    }
}

pub fn handle_middle_mouse_down(app: &mut App<'_>, row: u16, column: u16) {
    if let Some((msg, ch)) = mouse_pos_to_selpos(
        app.conversation_rect,
        &app.display_lines,
        app.scroll,
        row,
        column,
    ) {
        app.selection = Some(((msg, ch), (msg, ch)));
        app.selecting = true;
    }
}

pub fn handle_middle_mouse_drag(app: &mut App<'_>, row: u16, column: u16) {
    if app.selecting {
        if let Some((msg, ch)) = mouse_pos_to_selpos(
            app.conversation_rect,
            &app.display_lines,
            app.scroll,
            row,
            column,
        ) {
            if let Some((start_pos, _)) = app.selection {
                let a = start_pos;
                let b = (msg, ch);
                let (s, e) = if a <= b { (a, b) } else { (b, a) };
                app.selection = Some((s, e));
            } else {
                app.selection = Some(((msg, ch), (msg, ch)));
            }
        }
    }
}

/// Main event handler - processes all keyboard and mouse events
pub async fn handle_events(app: &mut App<'_>, event: Event, should_quit: &mut bool) {
    match event {
        Event::Key(key) => {
            if key.kind == crossterm::event::KeyEventKind::Press {
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
                            handle_ctrl_y(app).await;
                        }
                        KeyCode::Char('r') if matches!(app.mode, AppMode::Settings) => {
                            handle_ctrl_r(app).await;
                        }
                        KeyCode::Char('t') => {
                            handle_ctrl_t(app).await;
                        }
                        _ => {}
                    }
                } else {
                    match app.mode {
                        AppMode::Chat => match key.code {
                            KeyCode::Enter => {
                                handle_chat_enter(app);
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
                                        app.config.selected_model = app
                                            .models
                                            .items
                                            .get(selected_index)
                                            .map(|m| m.name.clone());
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
                                        app.config.selected_model = app
                                            .models
                                            .items
                                            .get(selected_index)
                                            .map(|m| m.name.clone());
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
        Event::Mouse(mouse_event) => match mouse_event.kind {
            MouseEventKind::ScrollUp => {
                handle_mouse_scroll_up(app, mouse_event.row, mouse_event.column);
            }
            MouseEventKind::ScrollDown => {
                handle_mouse_scroll_down(app, mouse_event.row, mouse_event.column);
            }
            MouseEventKind::Down(MouseButton::Left) => {
                handle_left_mouse_down(app, mouse_event.row, mouse_event.column);
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                handle_left_mouse_drag(app, mouse_event.row, mouse_event.column);
            }
            MouseEventKind::Up(MouseButton::Left) => {
                handle_left_mouse_up(app);
            }
            MouseEventKind::Down(MouseButton::Middle) => {
                handle_middle_mouse_down(app, mouse_event.row, mouse_event.column);
            }
            MouseEventKind::Drag(MouseButton::Middle) => {
                handle_middle_mouse_drag(app, mouse_event.row, mouse_event.column);
            }
            MouseEventKind::Up(MouseButton::Middle) => {
                handle_middle_mouse_up(app);
            }
            _ => {}
        },
        _ => {}
    }
}
