use arboard::Clipboard;
use tokio::task;

pub async fn copy_to_clipboard(text: String) {
    task::spawn_blocking(move || {
        if let Ok(mut clipboard) = Clipboard::new() {
            if let Err(e) = clipboard.set_text(text) {
                log::error!("Failed to set clipboard text: {}", e);
            }
        } else {
            log::error!("Failed to initialize clipboard.");
        }
    });
}