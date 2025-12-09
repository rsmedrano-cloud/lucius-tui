pub fn get_text_coordinates(
    layout_area: ratatui::layout::Rect,
    mouse_x: u16,
    mouse_y: u16,
) -> Option<(usize, usize)> {
    // For now, just a placeholder
    if mouse_y >= layout_area.y && mouse_y < layout_area.y + layout_area.height {
        let line_index = (mouse_y - layout_area.y) as usize;
        let char_index = (mouse_x - layout_area.x) as usize;
        Some((line_index, char_index))
    } else {
        None
    }
}