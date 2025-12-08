use std::io::Write;
use std::process::{Command, Stdio};

/// Copy content to system clipboard (Ctrl+V paste)
/// Tries multiple clipboard utilities in order of preference
pub fn copy_to_clipboard(content: &str) -> Result<(), String> {
    let candidates: Vec<(&str, Vec<&str>)> = vec![
        ("pbcopy", vec![]),                         // macOS
        ("wl-copy", vec![]),                        // Wayland
        ("xclip", vec!["-selection", "clipboard"]), // X11
        ("xsel", vec!["--clipboard", "--input"]),   // X11 alternative
        ("clip", vec![]),                           // Windows
    ];

    for (cmd, args) in candidates {
        match Command::new(cmd)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() {
                    if stdin.write_all(content.as_bytes()).is_ok() {
                        drop(stdin);
                        return Ok(());
                    }
                }
            }
            Err(_) => continue,
        }
    }
    Err("No clipboard command available (pbcopy, wl-copy, xclip, xsel, clip)".to_string())
}

/// Copy content to primary selection (X11 middle-click paste)
/// Only works on X11 systems with xclip or xsel
pub fn copy_to_primary(content: &str) -> Result<(), String> {
    let candidates = vec![
        ("xclip", vec!["-selection", "primary"]),
        ("xsel", vec!["--primary", "--input"]),
    ];

    for (cmd, args) in candidates {
        match Command::new(cmd)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() {
                    if stdin.write_all(content.as_bytes()).is_ok() {
                        drop(stdin);
                        return Ok(());
                    }
                }
            }
            Err(_) => continue,
        }
    }
    Err("No primary selection command available (xclip, xsel)".to_string())
}
