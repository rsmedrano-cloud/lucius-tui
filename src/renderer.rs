use crate::mouse::DisplayLine;
use ratatui::{
    prelude::{Color, Style},
    text::{Line, Span, Text},
};

/// Build display lines from chat history with word wrapping
/// Returns a vector of (message_index, start_char, chunk) tuples
pub fn build_display_lines(chat_history: &[String], inner_width: usize) -> Vec<DisplayLine> {
    let mut display_lines: Vec<DisplayLine> = Vec::new();
    for (msg_idx, message) in chat_history.iter().enumerate() {
        let chars: Vec<char> = message.chars().collect();
        if chars.is_empty() {
            display_lines.push((msg_idx, 0, String::new()));
            continue;
        }
        let mut start = 0usize;
        while start < chars.len() {
            let end = (start + inner_width).min(chars.len());
            let chunk: String = chars[start..end].iter().collect();
            display_lines.push((msg_idx, start, chunk));
            start = end;
        }
    }
    display_lines
}

/// Build styled Text with selection highlighting from display lines
/// Selection (if present) is rendered with dark gray background and white text
pub fn build_selection_text<'a>(
    display_lines: &'a [DisplayLine],
    selection: Option<((usize, usize), (usize, usize))>,
) -> Text<'a> {
    let mut text_lines: Vec<Line> = Vec::new();

    for (msg_idx, start_char, chunk) in display_lines {
        let mut spans_vec: Vec<Span> = Vec::new();
        if let Some(((s_msg, s_char), (e_msg, e_char))) = selection {
            if *msg_idx < s_msg || *msg_idx > e_msg {
                spans_vec.push(Span::raw(chunk.clone()));
            } else {
                let chunk_len = chunk.chars().count();
                // selection char indices in this message
                let sel_start = if *msg_idx == s_msg { s_char } else { 0 };
                let sel_end = if *msg_idx == e_msg {
                    e_char
                } else {
                    usize::MAX
                };
                // convert to chunk-local indices
                let chunk_start_abs = *start_char;
                let sel_start_in_chunk = if sel_start <= chunk_start_abs {
                    0
                } else {
                    sel_start - chunk_start_abs
                };
                let sel_end_in_chunk = if sel_end == usize::MAX {
                    chunk_len.saturating_sub(1)
                } else {
                    if sel_end < chunk_start_abs {
                        0
                    } else {
                        (sel_end - chunk_start_abs).min(chunk_len.saturating_sub(1))
                    }
                };
                if sel_start_in_chunk > 0 {
                    let before: String = chunk.chars().take(sel_start_in_chunk).collect();
                    spans_vec.push(Span::raw(before));
                }
                if sel_end_in_chunk >= sel_start_in_chunk && sel_start_in_chunk < chunk_len {
                    let sel_mid: String = chunk
                        .chars()
                        .skip(sel_start_in_chunk)
                        .take(sel_end_in_chunk - sel_start_in_chunk + 1)
                        .collect();
                    spans_vec.push(Span::styled(
                        sel_mid,
                        Style::default().bg(Color::DarkGray).fg(Color::White),
                    ));
                }
                let after_start = if sel_end_in_chunk + 1 < chunk_len {
                    sel_end_in_chunk + 1
                } else {
                    chunk_len
                };
                if after_start < chunk_len {
                    let after: String = chunk.chars().skip(after_start).collect();
                    spans_vec.push(Span::raw(after));
                }
            }
        } else {
            spans_vec.push(Span::raw(chunk.clone()));
        }
        text_lines.push(Line::from(spans_vec));
    }

    Text::from(text_lines)
}

/// Extract selected text from chat history
/// Builds a string from the selection across one or more messages
pub fn extract_selection_text(
    chat_history: &[String],
    selection: ((usize, usize), (usize, usize)),
) -> String {
    let (s_msg, s_char) = selection.0;
    let (e_msg, e_char) = selection.1;
    let mut parts: Vec<String> = vec![];

    if s_msg == e_msg {
        if let Some(msg) = chat_history.get(s_msg) {
            let chars: Vec<char> = msg.chars().collect();
            let start = s_char.min(chars.len());
            let end = e_char.min(chars.len().saturating_sub(1));
            if start <= end && end < chars.len() {
                parts.push(chars[start..=end].iter().collect());
            }
        }
    } else {
        // First part
        if let Some(msg) = chat_history.get(s_msg) {
            let chars: Vec<char> = msg.chars().collect();
            let start = s_char.min(chars.len());
            if start < chars.len() {
                parts.push(chars[start..].iter().collect());
            }
        }
        // Middle messages
        for idx in (s_msg + 1)..e_msg {
            if let Some(m) = chat_history.get(idx) {
                parts.push(m.clone());
            }
        }
        // Last part
        if let Some(msg) = chat_history.get(e_msg) {
            let chars: Vec<char> = msg.chars().collect();
            let end = e_char.min(chars.len().saturating_sub(1));
            if end < chars.len() {
                parts.push(chars[..=end].iter().collect());
            }
        }
    }

    parts.join("\n")
}

/// Render the UI frame based on current app state
pub fn render_frame(frame: &mut ratatui::Frame, app: &mut crate::app::App) {
    use crate::ui::{AppMode, Focus, ASCII_ART, HELP_MESSAGE};
    use ratatui::prelude::{Alignment, Constraint, Direction, Layout, Modifier};
    use ratatui::widgets::{Block, Borders, List, ListItem, Padding, Paragraph, Wrap};

    let area = frame.area();
    match app.mode {
        AppMode::Chat => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(7),
                    Constraint::Min(0),
                    Constraint::Length(1),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ])
                .split(area);

            let ascii_art = Paragraph::new(ASCII_ART).alignment(Alignment::Center);
            frame.render_widget(ascii_art, chunks[0]);

            let inner_width = chunks[1].width.saturating_sub(4) as usize;
            let display_lines = build_display_lines(&app.chat_history, inner_width);
            app.display_lines = display_lines.clone();

            let conversation_block = Block::default()
                .title("Conversation")
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .padding(Padding::new(1, 1, 1, 1));

            app.conversation_rect = Some(chunks[1]);

            let chat_area_height = chunks[1].height.saturating_sub(2) as usize;
            let num_lines_in_history = app.display_lines.len();

            let max_scroll_offset = if num_lines_in_history > chat_area_height {
                (num_lines_in_history - chat_area_height) as u16
            } else {
                0
            };

            app.scroll = app.scroll.min(max_scroll_offset);

            let history_text = build_selection_text(&app.display_lines, app.selection);
            let history = Paragraph::new(history_text)
                .wrap(Wrap { trim: true })
                .scroll((app.scroll, 0))
                .block(conversation_block);
            frame.render_widget(history, chunks[1]);

            let status_text = if let Some((msg, _)) = &app.status_message {
                msg.clone()
            } else {
                let lucius_md_count = if app.lucius_context.is_some() { 1 } else { 0 };
                let mcp_server_count = if app.mcp_request_tx.is_some() { 1 } else { 0 };
                format!(
                    "using: {} LUCIUS.md | {} MCP server",
                    lucius_md_count, mcp_server_count
                )
            };
            let status_line = Paragraph::new(status_text).style(if app.status_message.is_some() {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            });
            frame.render_widget(status_line, chunks[2]);

            frame.render_widget(&app.textarea, chunks[3]);

            let bottom_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[4]);

            let current_dir = std::env::current_dir()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|_| "Error getting dir".to_string());
            let dir_paragraph = Paragraph::new(format!("Dir: {}", current_dir))
                .style(Style::default().fg(Color::Blue));
            frame.render_widget(dir_paragraph, bottom_chunks[0]);

            let active_model_name = app
                .models
                .items
                .get(app.models.state.selected().unwrap_or(0))
                .map(|model| model.name.clone())
                .unwrap_or_else(|| "No model selected".to_string());
            let model_paragraph = Paragraph::new(format!("Model: {}", active_model_name))
                .alignment(Alignment::Right)
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

            let url_editor_block = Block::default().borders(Borders::ALL).title("Ollama URL");
            if let Focus::Url = app.focus {
                app.url_editor.set_block(
                    url_editor_block
                        .clone()
                        .border_style(Style::default().fg(Color::LightCyan)),
                );
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

            let models_block = Block::default().title("Models").borders(Borders::ALL);
            let items: Vec<ListItem> = app
                .models
                .items
                .iter()
                .map(|i| ListItem::new(i.name.as_str()))
                .collect();
            let list = List::new(items)
                .block(if let Focus::Models = app.focus {
                    models_block.border_style(Style::default().fg(Color::LightCyan))
                } else {
                    models_block
                })
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                .highlight_symbol(">> ");

            frame.render_stateful_widget(list, chunks[2], &mut app.models.state);
        }
        AppMode::Help => {
            let help_block = Block::default().title("Help").borders(Borders::ALL);
            let help_paragraph = Paragraph::new(HELP_MESSAGE)
                .wrap(Wrap { trim: true })
                .block(help_block);
            frame.render_widget(help_paragraph, area);
        }
    }
}
