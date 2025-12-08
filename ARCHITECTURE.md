# Lucius Architecture

This document describes the technical architecture of Lucius, including design decisions, data flow, and module interactions.

## Overview

Lucius is a **modal TUI application** with an **event-driven architecture**. All user interactions flow through a centralized event handler, which dispatches to specialized handlers that modify application state. The rendering layer then displays the updated state.

```
┌─────────────────────┐
│   main.rs           │
│  (Event Loop)       │
└──────────┬──────────┘
           │
           ├─→ event::poll() / read()
           │
           ├─→ handlers::handle_events()
           │   ├─→ handle_ctrl_y()
           │   ├─→ handle_ctrl_r()
           │   ├─→ handle_mouse_*()
           │   └─→ ... (more handlers)
           │
           ├─→ renderer::render_frame()
           │   ├─→ render Chat mode
           │   ├─→ render Settings mode
           │   └─→ render Help mode
           │
           └─→ Loop
```

## Core Data Structures

### App State

```rust
pub struct App {
    // UI State
    pub mode: AppMode,
    pub focus: Focus,
    pub textarea: TextArea,

    // Chat State
    pub chat_history: Vec<String>,
    pub display_lines: Vec<DisplayLine>,
    pub selection: Option<((usize, usize), (usize, usize))>,
    pub selecting: bool,
    pub scroll: u16,

    // Model State
    pub models: StatefulList<Model>,
    pub config: Config,
    pub status: bool,
    pub status_message: Option<(String, Instant)>,

    // Channels
    pub response_rx: mpsc::UnboundedReceiver<String>,
    pub mcp_request_tx: Option<mpsc::UnboundedSender<McpRequest>>,

    // Context
    pub lucius_context: Option<String>,
}
```

### Display Lines

Text wrapping is handled by breaking messages into "display lines":

```rust
pub type DisplayLine = (usize, usize, String);
// (message_index, start_char_index, chunk_text)
```

Example:
```
Message 0: "Hello world, this is a long message"
Message 1: "This is another message"

With width=15:
Display Line 0: (0, 0, "Hello world, t")
Display Line 1: (0, 14, "his is a long ")
Display Line 2: (0, 28, "message")
Display Line 3: (1, 0, "This is another")
Display Line 4: (1, 15, " message")
```

### Selection Coordinates

Selection is stored as character indices within messages:

```rust
type Selection = ((msg_idx, char_idx), (msg_idx, char_idx));
// ((start_message, start_char), (end_message, end_char))
```

This enables:
- Precise clipboard operations
- Cross-message selections
- Character-level text copying
- Independent of terminal width/wrapping

## Event Handling

### Event Flow

1. **main.rs** polls for events every 50ms
2. **handlers::handle_events()** dispatches based on event type
3. **Specialized handlers** modify app state
4. **renderer::render_frame()** displays the new state

### Handler Categories

#### Keyboard Handlers (Ctrl+*)

- `handle_ctrl_h()`: Toggle Help mode
- `handle_ctrl_q()`: Set `should_quit = true`
- `handle_ctrl_s()`: Switch to Settings mode
- `handle_ctrl_l()`: Clear chat history
- `handle_ctrl_y()`: Copy selected text or last response
- `handle_ctrl_r()`: Refresh models from Ollama
- `handle_ctrl_t()`: List MCP tools

#### Mode-Specific Handlers

- Chat mode: `KeyCode::Enter` → `handle_chat_enter()`
- Settings/Url: `Tab` → switch focus, `Enter` → confirm
- Settings/Models: `Up`/`Down` → navigate, `Enter` → select

#### Mouse Handlers

- `handle_mouse_scroll_up/down()`: Scroll conversation
- `handle_left_mouse_down()`: Start selection
- `handle_left_mouse_drag()`: Extend selection
- `handle_left_mouse_up()`: Copy to clipboard
- `handle_middle_mouse_*()`: Same as left, but primary selection too

## Text Selection & Clipboard

### Selection Process

```
1. User left-clicks at position (row=15, col=30)
   ↓
2. mouse::mouse_pos_to_selpos() converts to (msg_idx=2, char_idx=5)
   ↓
3. app.selection = Some(((2, 5), (2, 5)))
   ↓
4. User drags to (row=16, col=45) → (msg_idx=3, char_idx=12)
   ↓
5. app.selection = Some(((2, 5), (3, 12)))
   ↓
6. User releases mouse
   ↓
7. handle_left_mouse_up() calls:
   - renderer::extract_selection_text() → gets text
   - clipboard::copy_to_clipboard() → spawns process
   ↓
8. Status message: "Copied to clipboard!"
```

### Clipboard Implementation

Multi-platform non-blocking clipboard:

```rust
pub fn copy_to_clipboard(text: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    let cmd = ("pbcopy", vec![]);

    #[cfg(all(unix, not(target_os = "macos")))]
    let cmd = if has_command("wl-copy") {
        ("wl-copy", vec!["-p"]) // Wayland
    } else {
        ("xclip", vec!["-selection", "clipboard"]) // X11
    };

    let mut child = Command::new(cmd.0)
        .args(&cmd.1)
        .stdin(Stdio::piped())
        .spawn()?;

    child.stdin.unwrap().write_all(text.as_bytes())?;
    // Process runs in background, doesn't block UI
}
```

## Rendering Pipeline

### Render Frame Structure

```
┌────────────────────────────┐
│  ASCII Art (7 lines)       │
├────────────────────────────┤
│                            │
│  Conversation              │ ← Scrollable
│  (wrapped with selection   │
│   highlighting)            │
│                            │
├────────────────────────────┤
│ Status: context/MCP info   │
├────────────────────────────┤
│ Textarea (input)           │
├────────────────────────────┤
│ Dir: ... | Model: ...      │
└────────────────────────────┘
```

### Word Wrapping

Text wrapping happens in `renderer::build_display_lines()`:

```rust
pub fn build_display_lines(
    chat_history: &[String],
    inner_width: usize
) -> Vec<DisplayLine> {
    // For each message, break into chunks of width `inner_width`
    // Return (message_index, start_char, chunk_text)
}
```

This allows the selection system to work independently of viewport width.

### Selection Highlighting

Styled rendering in `renderer::build_selection_text()`:

```rust
if selection_spans_this_line {
    // Render before selection: normal text
    Span::raw(before_selection)

    // Render selected text: dark gray background
    Span::styled(selected_text,
        Style::default()
            .bg(Color::DarkGray)
            .fg(Color::White)
    )

    // Render after selection: normal text
    Span::raw(after_selection)
}
```

## LLM Integration

### Message Flow

```
User Input
   ↓
app.chat_history.push("You: " + input)
   ↓
Spawn async task with tokio::spawn()
   ↓
llm::handle_llm_turn() {
   - Load LUCIUS.md context
   - Call Ollama streaming API
   - Process tool calls (MCP)
   - Stream response to response_rx
}
   ↓
Main loop receives from response_rx
   ↓
app.chat_history.push("Lucius: " + response)
   ↓
Render updated conversation
```

### Streaming Responses

Ollama returns Server-Sent Events (SSE):

```rust
let response = reqwest::Client::new()
    .post(&format!("{}/api/chat", url))
    .json(&payload)
    .send()
    .await?
    .bytes_stream();

for await chunk in response {
    let json: OllamaChunkResponse = serde_json::from_slice(&chunk)?;
    if let Some(part) = json.message.content {
        tx.send(part)?; // Send to response_rx
    }
}
```

## Model Context Protocol (MCP)

### MCP Server Communication

```
Handle Ctrl+T
   ↓
handlers::handle_ctrl_t() {
   Send McpRequest to mcp_request_tx
   Wait for response via oneshot channel
}
   ↓
MCP background task receives request
   ↓
mcp::process_mcp_request() {
   - Start/connect to MCP server
   - Call requested method (e.g., "list_tools")
   - Return result via oneshot_tx
}
   ↓
Status message updated with result
```

## Configuration System

### Config File Location

- **Linux/macOS**: `~/.config/lucius/config.toml`
- **Windows**: `%APPDATA%\lucius\config.toml`

### Config Contents

```toml
[ui]
ollama_url = "http://192.168.1.42:11434"

[model]
selected = "llama2"
```

### Loading Strategy

1. Check if config file exists
2. If missing, create with defaults
3. Load URL and model from file
4. Allow in-app overrides (Settings screen)
5. Auto-save when changed

## Context Engine (LUCIUS.md)

### Discovery Algorithm

```
Start at current working directory
   ↓
While directory != root {
   if LUCIUS.md exists:
      Load and use it
      Break

   parent_dir = parent_of(current_dir)
}
```

### Usage in Prompt

```
"You are a helpful assistant. Here is the project context:

" + lucius_context + "

User's message: " + user_input
```

## Performance Considerations

### Memory Usage

Target: < 20MB RAM

- Chat history: O(n) messages
- Display lines: O(m) where m = total wrapped lines
- UI state: fixed size

### CPU Usage

- Event polling: 50ms sleep, minimal CPU
- Rendering: happens every frame (~60 FPS, 16ms per frame)
- Background tasks: async, non-blocking

### Optimization Techniques

1. **Lazy display line generation**: Only computed on render
2. **Non-blocking I/O**: All I/O in background tasks
3. **Single-threaded event loop**: No contention
4. **Incremental rendering**: Only changed parts

## Testing Strategy

### Unit Tests

- Text selection/extraction logic
- Clipboard operations (mocked)
- Config loading/saving

### Integration Tests

- Full event flow
- Rendering output
- State transitions

### Manual Testing

- Different terminals
- Different OS platforms
- Clipboard tools availability
- Mouse interaction

## Future Architecture Improvements

1. **Theme System**: Allow customizable colors/styles
2. **Plugin System**: Load external handlers
3. **Configuration Profiles**: Save different settings
4. **Logging Framework**: Better debugging support
5. **State Persistence**: Save conversation history
6. **Keyboard Macro System**: Record/playback key sequences

## Module Dependency Graph

```
main.rs
   ├─→ handlers.rs
   │   ├─→ app.rs
   │   ├─→ clipboard.rs
   │   ├─→ llm.rs
   │   ├─→ mcp.rs
   │   ├─→ mouse.rs
   │   ├─→ renderer.rs
   │   └─→ ui.rs
   ├─→ renderer.rs
   │   ├─→ app.rs
   │   ├─→ mouse.rs
   │   └─→ ui.rs
   ├─→ app.rs
   │   ├─→ config.rs
   │   ├─→ context.rs
   │   └─→ ui.rs
   ├─→ llm.rs
   │   ├─→ mcp.rs
   │   └─→ context.rs
   └─→ ... (other modules)

Legend:
X → Y means "X imports from Y"
Minimal circular dependencies
```

## Conclusion

Lucius's architecture prioritizes:
- **Clarity**: Clear separation of concerns
- **Performance**: Non-blocking, efficient operations
- **Extensibility**: Modular design for new features
- **Maintainability**: Focused modules with single responsibilities
