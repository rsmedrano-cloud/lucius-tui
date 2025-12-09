use arboard::Clipboard;

pub fn copy_to_clipboard(text: String) {
    if let Ok(mut clipboard) = Clipboard::new() {
        if let Err(e) = clipboard.set_text(text) {
            log::error!("Failed to set clipboard text: {}", e);
        }
    } else {
        log::error!("Failed to initialize clipboard.");
    }
}
