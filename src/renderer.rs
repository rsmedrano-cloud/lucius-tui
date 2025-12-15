use ratatui::{
    prelude::{Frame, Layout, Direction, Constraint, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap, Padding},
    text::{Line, Text},
    layout::Alignment,
    style::{Color, Modifier},
};
use termimad::MadSkin;

use crate::app::{App, SharedState};
use crate::ui::{AppMode, Focus, ConfirmationModal, HELP_MESSAGE, ASCII_ART};

pub fn draw_ui(f: &mut Frame, app: &mut App, state: &SharedState) {
    let area = f.area();
    
    // Render based on the current mode from the shared state.
    match &state.mode {
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
            let ascii_art = Paragraph::new(ASCII_ART).alignment(Alignment::Center);
            f.render_widget(ascii_art, chunks[0]);
            
            // Conversation History from shared state
            let history_text: String = state.chat_history.join("\n");
            let markdown_text = MadSkin::default().term_text(&history_text).to_string();

            let conversation_block = Block::default()
                .title("Conversation")
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .padding(Padding::new(1, 1, 1, 1));

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
            f.render_widget(history, chunks[1]);
            app.conversation_area = chunks[1];

            // Status line from shared state
            let status_text = if let Some((msg, _)) = &state.status_message {
                msg.clone()
            } else {
                let lucius_md_count = if state.lucius_context.is_some() { 1 } else { 0 };
                let mcp_server_count = if state.redis_conn.is_some() { 1 } else { 0 };
                format!("using: {} LUCIUS.md | {} MCP server", lucius_md_count, mcp_server_count)
            };
            let status_line = Paragraph::new(status_text)
                .style(if state.status_message.is_some() {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::DarkGray)
                });
            f.render_widget(status_line, chunks[2]);

            // UI-specific widgets from App
            f.render_widget(&app.textarea, chunks[3]);
            
            let bottom_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[4]);

            let current_dir = std::env::current_dir()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|_| "Error getting dir".to_string());
            let dir_paragraph = Paragraph::new(format!("Dir: {}", current_dir))
                .style(Style::default().fg(Color::Blue));
            f.render_widget(dir_paragraph, bottom_chunks[0]);

            let active_model_name = state.models.get(app.model_list_state.selected().unwrap_or(0))
                .map(|model| model.name.clone())
                .unwrap_or_else(|| "No model selected".to_string());
            let model_paragraph = Paragraph::new(format!("Model: {}", active_model_name))
                .alignment(Alignment::Right)
                .style(Style::default().fg(Color::LightCyan));
            f.render_widget(model_paragraph, bottom_chunks[1]);
        }
        AppMode::Settings => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Ollama URL editor
                    Constraint::Length(3), // MCP Redis Host editor
                    Constraint::Length(3), // Status
                    Constraint::Min(0),    // Models list
                ])
                .split(area);

            let ollama_url_editor_block = Block::default().borders(Borders::ALL).title("Ollama URL");
            if let Focus::Url = app.focus {
                app.url_editor.set_block(ollama_url_editor_block.clone().border_style(Style::default().fg(Color::LightCyan)));
            } else {
                app.url_editor.set_block(ollama_url_editor_block);
            }
            f.render_widget(app.url_editor.widget(), chunks[0]);

            let mcp_url_editor_block = Block::default().borders(Borders::ALL).title("MCP Redis Host");
            if let Focus::McpUrl = app.focus {
                app.mcp_url_editor.set_block(mcp_url_editor_block.clone().border_style(Style::default().fg(Color::LightCyan)));
            } else {
                app.mcp_url_editor.set_block(mcp_url_editor_block);
            }
            f.render_widget(app.mcp_url_editor.widget(), chunks[1]);


            let (status_text, status_color) = if state.status {
                ("Status: Connected", Color::Green)
            } else {
                ("Status: Disconnected", Color::Red)
            };
            let status = Paragraph::new(status_text)
                .style(Style::default().fg(status_color))
                .block(Block::default().title("Status").borders(Borders::ALL));
            f.render_widget(status, chunks[2]);
            
            let models_block = Block::default().title("Models").borders(Borders::ALL);
            let items: Vec<ListItem> = state.models.iter().map(|i| ListItem::new(i.name.as_str())).collect();
            let list = List::new(items)
                .block(if let Focus::Models = app.focus {
                    models_block.border_style(Style::default().fg(Color::LightCyan))
                } else {
                    models_block
                })
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                .highlight_symbol(">>");

            // Correctly render the stateful widget
            f.render_stateful_widget(list, chunks[3], &mut app.model_list_state);
        }
        AppMode::Help => {
            let help_block = Block::default().title("Help").borders(Borders::ALL);
            let help_paragraph = Paragraph::new(HELP_MESSAGE).wrap(Wrap { trim: true }).block(help_block);
            f.render_widget(help_paragraph, area);
        }
        AppMode::Confirmation(ConfirmationModal::ExecuteTool { ref tool_call, .. }) => {
            // Re-draw the chat UI in the background
            draw_ui(f, app, &state); // This might not be perfect, but it shows the context

            let modal_width = 60;
            let modal_height = 8;
            let popup_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(modal_height),
                    Constraint::Min(0),
                ])
                .split(area);

            let popup_area = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(modal_width),
                    Constraint::Min(0),
                ])
                .split(popup_layout[1])[1];

            let text: Vec<Line> = vec![
                Line::from("Execute Command?"),
                Line::from(""),
                Line::from(format!("Tool: {}", tool_call.tool.clone())),
                Line::from(format!("Params: {}", tool_call.params.clone())),
                Line::from(""),
                Line::from("Press 'y' to confirm, 'n' to cancel."),
            ];
            let block = Block::default()
                .title("CONFIRM ACTION")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::DarkGray).fg(Color::White));
            let paragraph = Paragraph::new(text).block(block).alignment(Alignment::Center).wrap(Wrap { trim: true });
            f.render_widget(paragraph, popup_area);
        }
    }
}