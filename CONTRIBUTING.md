# Contributing to Lucius

Thank you for your interest in contributing to Lucius! This guide will help you get started with development.

## Development Setup

### Prerequisites

- Rust 1.70.0+ (use `rustup` to install/update)
- Ollama instance running locally
- Your favorite code editor (VS Code + rust-analyzer recommended)

### Initial Setup

1. **Clone the repository**:
   ```bash
   git clone https://github.com/rsmedrano-cloud/lucius-tui.git
   cd lucius-tui
   ```

2. **Build the project**:
   ```bash
   cargo build
   ```

3. **Run in development mode**:
   ```bash
   cargo run --bin lucius
   ```

## Code Organization

The project is organized into focused modules:

```
src/
├── main.rs          # Event loop and terminal orchestration (99 lines)
├── handlers.rs      # All event handling logic (461 lines)
├── renderer.rs      # UI rendering for all modes (301 lines)
├── app.rs           # Application state structure
├── llm.rs           # Ollama integration
├── mcp.rs           # MCP server communication
├── clipboard.rs     # Multi-platform clipboard operations
├── mouse.rs         # Mouse utilities and coordinate mapping
├── ui.rs            # UI types and constants
├── config.rs        # Configuration management
└── context.rs       # LUCIUS.md context engine
```

## Architecture Overview

### Event Flow

1. **main.rs**: Polls for events
2. **handlers.rs::handle_events()**: Dispatches to specific handlers
3. **Specialized handlers**: Process the event and update app state
4. **renderer.rs::render_frame()**: Renders the updated state

### Key Design Patterns

- **Non-blocking I/O**: Long operations (LLM calls, clipboard ops) run in background tasks
- **Immutable App Structure**: State is passed mutably to handlers, which update it in-place
- **Character-Level Selection**: Selection coordinates are `(message_index, char_index)` tuples

## Making Changes

### Adding a New Keyboard Shortcut

1. Add the handler function in `handlers.rs`:
   ```rust
   pub async fn handle_ctrl_x(app: &mut App<'_>) {
       // Your logic here
   }
   ```

2. Add the dispatcher in `handlers.rs::handle_events()`:
   ```rust
   KeyCode::Char('x') => {
       handle_ctrl_x(app).await;
   }
   ```

3. Update keybindings in `README.md`

### Adding a New UI Mode

1. Add variant to `AppMode` enum in `ui.rs`:
   ```rust
   pub enum AppMode {
       Chat,
       Settings,
       Help,
       NewMode,  // ← Add here
   }
   ```

2. Add rendering logic in `renderer.rs::render_frame()`:
   ```rust
   AppMode::NewMode => {
       // Render your UI here
   }
   ```

3. Add event handling in `handlers.rs::handle_events()` if needed

### Modifying Rendering

- All UI rendering is in `renderer.rs`
- Use Ratatui widgets and layout system
- Keep rendering logic separate from event handling

### Working with State

The `App` struct in `app.rs` holds all application state. When adding new state:

1. Add field to `App` struct
2. Initialize in `App::new()`
3. Update in relevant handlers
4. Render if needed in `renderer.rs`

## Testing

### Running Tests

```bash
cargo test
```

### Manual Testing

1. **Test with different terminal emulators**:
   - GNOME Terminal
   - Alacritty
   - Kitty
   - iTerm (macOS)

2. **Test clipboard operations on your platform**:
   - Copy text selections
   - Use `Ctrl+Y` to copy responses

3. **Test mouse interactions**:
   - Scroll in conversation
   - Select text with click & drag
   - Middle-click primary selection

## Code Style

- Follow standard Rust conventions (enforced by `rustfmt`)
- Run formatter before committing:
  ```bash
  cargo fmt
  ```

- Run linter:
  ```bash
  cargo clippy
  ```

## Debugging

### Enable Debug Logging

```bash
RUST_LOG=debug cargo run --bin lucius
```

Logs are written to `lucius.log` in the current directory.

### Common Debug Scenarios

**Q: UI not updating**
- Check that state is being modified in handlers
- Verify `renderer.rs` is reading the updated state
- Make sure event handlers are being called

**Q: Mouse events not working**
- Check `mouse.rs::mouse_pos_to_selpos()` is returning correct coordinates
- Verify `app.conversation_rect` is set correctly in renderer
- Test in a different terminal emulator

**Q: Clipboard operations failing**
- Verify appropriate tool is installed (`xclip`, `wl-copy`, etc.)
- Check that spawned process has exit code 0
- Try running tool manually: `echo "test" | xclip -selection clipboard`

## Performance Considerations

1. **Non-blocking Operations**: Keep event handler latency < 1ms
2. **Background Tasks**: Use `tokio::spawn()` for long operations
3. **Memory**: Target < 20MB RAM usage (use `top` or similar to monitor)

## Submitting Changes

1. **Create a feature branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes** following the guidelines above

3. **Test thoroughly**:
   ```bash
   cargo build
   cargo test
   cargo clippy
   cargo fmt
   ```

4. **Write a clear commit message**:
   ```
   Add feature: Brief description

   - Detailed explanation
   - What changed and why
   ```

5. **Push and create a Pull Request**:
   ```bash
   git push origin feature/your-feature-name
   ```

## Architecture Decisions

### Why Event-Driven?

The centralized `handle_events()` function makes it easy to:
- Track all user interactions
- Debug event handling issues
- Add global keybindings
- Prevent event handling conflicts

### Why Separate Rendering?

Keeping rendering in `renderer.rs`:
- Makes UI changes isolated
- Easy to test UI logic
- Supports adding new themes
- Simplifies testing event handlers without rendering

### Why Character-Level Selection?

Instead of line-based selection:
- Supports copy-pasting specific portions
- Works seamlessly across message boundaries
- Natural for text-based interfaces
- Enables precise clipboard operations

## Resources

- **Ratatui Docs**: https://docs.rs/ratatui/latest/ratatui/
- **Tokio Async Runtime**: https://tokio.rs/
- **Crossterm Terminal**: https://docs.rs/crossterm/latest/crossterm/

## Questions?

Open an issue on GitHub for questions about the codebase or how to contribute.

Happy coding! 🚀
