use crate::mcp::ToolCall;

#[derive(Clone)]
pub enum AppMode {
    Chat,
    Settings,
    Help,
    Confirmation(ConfirmationModal),
}

impl PartialEq for AppMode {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (AppMode::Chat, AppMode::Chat) => true,
            (AppMode::Settings, AppMode::Settings) => true,
            (AppMode::Help, AppMode::Help) => true,
            (AppMode::Confirmation(a), AppMode::Confirmation(b)) => a == b,
            _ => false,
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum Focus {
    Url,
    Models,
}

pub enum ConfirmationModal {
    ExecuteTool {
        tool_call: ToolCall,
        confirm_tx: Option<tokio::sync::oneshot::Sender<bool>>,
    },
}

impl Clone for ConfirmationModal {
    fn clone(&self) -> Self {
        match self {
            ConfirmationModal::ExecuteTool { tool_call, .. } => {
                ConfirmationModal::ExecuteTool {
                    tool_call: tool_call.clone(),
                    confirm_tx: None, // Can't clone the sender
                }
            }
        }
    }
}

impl PartialEq for ConfirmationModal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ConfirmationModal::ExecuteTool { tool_call: a, .. }, ConfirmationModal::ExecuteTool { tool_call: b, .. }) => a == b,
        }
    }
}

pub const HELP_MESSAGE: &str = r#"
--- Help ---
Ctrl+H: Toggle Help
Ctrl+S: Toggle Settings
Ctrl+Q: Quit
Ctrl+L: Clear Chat
Ctrl+Y: Yank (Copy) Last Response
Ctrl+T: MCP Status
Esc: Interrupt current stream (if any)
Mouse Scroll: Scroll chat history
Shift + Mouse Drag: Select text for copying
Enter: Send message (Chat mode), Select model (Settings mode)
Tab: Switch focus (Settings mode)
Ctrl+R: Refresh models (Settings mode)
Esc: Go to Chat (Settings mode)
-----------------
"#;

pub const ASCII_ART: &str = r#"
 _               _              ____ _     ___ 
| |   _   _  ___(_)_   _ ___   / ___| |   |_ _|
| |  | | | |/ __| | | | / __| | |   | |    | |
| |__| |_| | (__| | |_| \__ \ | |___| |___ | |
|_____\__,_|\___|_|\__,_|___/  \____|_____|___|
"#;