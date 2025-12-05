use std::fs;

const LUCIUS_CONTEXT_FILENAME: &str = "LUCIUS.md";

/// Traverses parent directories starting from the current working directory
/// to find a file named `LUCIUS.md`.
/// If found, its content is read and returned as a String.
/// Returns None if the file is not found or cannot be read.
pub fn load_lucius_context() -> Option<String> {
    let mut current_path = std::path::PathBuf::from(std::env::current_dir().ok()?);

    loop {
        let potential_path = current_path.join(LUCIUS_CONTEXT_FILENAME);
        if potential_path.exists() && potential_path.is_file() {
            return fs::read_to_string(potential_path).ok();
        }

        // If we are at the root, stop
        if !current_path.pop() {
            break;
        }
    }
    None
}
