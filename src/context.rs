use std::fs;

const LUCIUS_CONTEXT_FILENAME: &str = "LUCIUS.md";
const DEFAULT_LUCIUS_CONTEXT: &str = r#"
# Lucius AI Assistant Context

You are Lucius, a helpful AI assistant. Respond concisely and accurately.
"#;

/// Traverses parent directories starting from the current working directory
/// to find a file named `LUCIUS.md`.
/// If found, its content is read and returned as a String.
/// If not found, a default `LUCIUS.md` is created in the current working directory,
/// and its content is returned.
/// Returns None if creation fails or cannot be read.
pub fn load_lucius_context() -> Option<String> {
    let mut current_path = std::path::PathBuf::from(std::env::current_dir().ok()?);
    let initial_cwd = current_path.clone(); // Store initial CWD for default creation

    loop {
        let potential_path = current_path.join(LUCIUS_CONTEXT_FILENAME);
        if potential_path.exists() && potential_path.is_file() {
            return fs::read_to_string(potential_path).ok();
        }

        // If we are at the root, stop
        if !current_path.pop() {
            // If we've reached the root and not found, create a default in initial CWD
            let default_path = initial_cwd.join(LUCIUS_CONTEXT_FILENAME);
            log::info!("LUCIUS.md not found. Creating default at: {}", default_path.display());
            if let Err(e) = fs::write(&default_path, DEFAULT_LUCIUS_CONTEXT.trim()) {
                log::error!("Failed to create default LUCIUS.md at {}: {}", default_path.display(), e);
                return None; // Return None if creation fails
            }
            return fs::read_to_string(default_path).ok(); // Read and return content of newly created file
        }
    }
}
