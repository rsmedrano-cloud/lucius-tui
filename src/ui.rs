use ratatui::widgets::ListState;

pub enum AppMode {
    Chat,
    Settings,
    Help,
}

pub enum Focus {
    Url,
    Models,
}

pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T> StatefulList<T> {
    pub fn new(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

pub const HELP_MESSAGE: &str = r#"
--- Help ---
Ctrl+H: Toggle Help
Ctrl+S: Toggle Settings
Ctrl+Q: Quit
Ctrl+L: Clear Chat
Ctrl+Y: Yank (Copy) Last Response
Ctrl+T: List MCP Tools
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
