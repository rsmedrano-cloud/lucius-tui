use ratatui::layout::Rect;

/// Data structure to hold display lines for rendering and mouse position mapping
/// Each tuple contains: (message_index, start_char_in_message, rendered_chunk)
pub type DisplayLine = (usize, usize, String);

/// Selection represents a character-level selection across one or more messages
/// Each tuple contains: (message_index, character_index)
pub type Selection = ((usize, usize), (usize, usize));

/// Check if mouse position is within the conversation area
pub fn cursor_in_conversation(rect: Option<Rect>, mouse_row: u16, mouse_col: u16) -> bool {
    if let Some(rect) = rect {
        mouse_col >= rect.x
            && mouse_col < rect.x + rect.width
            && mouse_row >= rect.y
            && mouse_row < rect.y + rect.height
    } else {
        false
    }
}

/// Map mouse position to character selection coordinates
/// Returns (message_index, character_index) in the chat history
pub fn mouse_pos_to_selpos(
    rect: Option<Rect>,
    display_lines: &[DisplayLine],
    scroll: u16,
    mouse_row: u16,
    mouse_col: u16,
) -> Option<(usize, usize)> {
    if let Some(rect) = rect {
        let inner_top = rect.y.saturating_add(2);
        let inner_bottom = rect.y + rect.height.saturating_sub(2);
        if mouse_row < inner_top || mouse_row >= inner_bottom {
            return None;
        }
        let relative = (mouse_row - inner_top) as usize;
        let line_index = scroll as usize + relative;
        if line_index < display_lines.len() {
            let (msg_idx, char_start, ref chunk) = &display_lines[line_index];
            let inner_left = rect.x.saturating_add(2);
            let rel_col = if mouse_col < inner_left {
                0
            } else {
                (mouse_col - inner_left) as usize
            };
            let mut char_idx = char_start + rel_col;
            let chunk_len = chunk.chars().count();
            if rel_col > chunk_len {
                char_idx = char_start + chunk_len;
            }
            Some((*msg_idx, char_idx))
        } else {
            None
        }
    } else {
        None
    }
}
