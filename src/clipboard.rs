use arboard::Clipboard;
use tokio::task;

pub async fn copy_to_clipboard(text: String) {
    task::spawn_blocking(move || {
        if let Ok(mut clipboard) = Clipboard::new() {
            if let Err(e) = clipboard.set_text(text) {
                log::error!("Failed to set clipboard text: {}", e);
            }
            // Keep the clipboard alive to serve the content
            // This will block the spawned_blocking thread until new content is copied or the app closes.
            // This is generally safe as it's a dedicated thread for clipboard ownership.
            let _ = clipboard.wait(); 
        } else {
            log::error!("Failed to initialize clipboard.");
        }
    });
}
