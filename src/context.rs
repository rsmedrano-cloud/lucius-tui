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
    let cwd = std::env::current_dir().ok()?;
    load_lucius_context_from(cwd)
}

/// Variant of `load_lucius_context` that starts searching from `start_path`.
/// This is useful for tests and other non-process-wide searches.
pub fn load_lucius_context_from(start_path: std::path::PathBuf) -> Option<String> {
    let mut current_path = start_path;
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
            log::info!(
                "LUCIUS.md not found. Creating default at: {}",
                default_path.display()
            );
            if let Err(e) = fs::write(&default_path, DEFAULT_LUCIUS_CONTEXT.trim()) {
                log::error!(
                    "Failed to create default LUCIUS.md at {}: {}",
                    default_path.display(),
                    e
                );
                return None; // Return None if creation fails
            }
            return fs::read_to_string(default_path).ok(); // Read and return content of newly created file
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_find_parent_lucius() {
        // Create temp dir structure: parent/child
        let parent = tempdir().unwrap();
        let parent_path = parent.path().to_path_buf();
        let child_path = parent_path.join("child");
        fs::create_dir_all(&child_path).unwrap();

        // Write LUCIUS.md in parent
        let lucius_path = parent_path.join(LUCIUS_CONTEXT_FILENAME);
        fs::write(&lucius_path, "# Test Lucius Parent").unwrap();

        // We won't change the process CWD - use helper starting from child path

        // Load context (should find parent file)
        println!("Parent LUCIUS path: {}", lucius_path.display());
        println!("Parent LUCIUS exists?: {}", lucius_path.exists());
        let loaded = load_lucius_context_from(child_path.clone()).unwrap_or_default();
        assert!(loaded.contains("Test Lucius Parent"));

        // No global CWD mutation: nothing to restore
    }

    #[test]
    fn test_create_default_lucius() {
        let temp = tempdir().unwrap();
        let temp_path = temp.path().to_path_buf();

        // Use helper starting from the temp directory

        // Ensure LUCIUS.md does not exist
        let path = temp_path.join(LUCIUS_CONTEXT_FILENAME);
        if path.exists() {
            fs::remove_file(&path).unwrap();
        }

        println!("Temp path: {}", temp_path.display());
        let loaded = load_lucius_context_from(temp_path.clone()).unwrap_or_default();
        assert!(loaded.contains("Lucius"));
        // Expect file to exist now
        assert!(path.exists());

        // No global CWD mutation: nothing to restore
    }
}
