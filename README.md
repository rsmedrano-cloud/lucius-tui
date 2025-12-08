# Lucius CLI (Rust)

A blazing fast, lightweight (sub-20MB RAM) TUI for local LLMs, written in Rust.

## Features

- **Lightweight & Fast**: Built with Rust and `ratatui` for minimal resource usage and a responsive feel.
- **Context Engine (`LUCIUS.md`)**: Automatically finds and uses a `LUCIUS.md` file in your project's directory hierarchy to provide persistent system-level context to the LLM.
- **Tool-use Loop**: Enables the LLM to interact with external tools (MCP servers) by generating tool calls, executing them, and incorporating the results into its responses.
- **Clipboard Integration**: Easily copy the last response from Lucius to the system clipboard using `Ctrl+Y`.
- **Mouse-Driven Text Selection**: Click and drag to select text in the conversation history. Auto-copies to clipboard on release (left-click) or clipboard + primary selection (middle-click).
- **Model Management**: Switch between different local models, see connection status, and refresh the model list from within the UI.
- **Persistent Configuration**: Remembers your Ollama URL and selected model between sessions.
- **Modern UI**: A clean interface with rounded borders, padded text, and dynamic information display.

## Keybindings

| Key                       | Action                                       |
| ------------------------- | -------------------------------------------- |
| `Ctrl+H`                  | Toggle the help screen.                      |
| `Ctrl+S`                  | Switch to the Settings screen.               |
| `Ctrl+Q`                  | Quit the application.                        |
| `Ctrl+L`                  | Clear the chat history.                      |
| `Ctrl+Y`                  | Copy selected text or last response to clipboard. |
| `Ctrl+R`                  | (Settings mode) Refresh available models from Ollama. |
| `Ctrl+T`                  | List available MCP tools in the status bar. |
| `Esc`                     | Exit modal screens (Help/Settings) or interrupt a streaming response. |
| `Enter`                   | Send the message in the input box.           |
| `Tab`                     | In Settings, switch focus between inputs.    |
| **Mouse Scroll**          | Scroll the conversation history.             |
| **Left-Click & Drag**     | Select text in conversation (auto-copies on release). |
| **Middle-Click & Drag**   | Select text (copies to clipboard + primary selection on release). |


## UI/UX Enhancements

*   **Dynamic Status Line**: A status line is displayed between the conversation and input box. It shows whether a `LUCIUS.md` file is in use, provides feedback for actions like copying to the clipboard, and indicates the status of the MCP server.
*   **Bottom Bar Information**: The bottom of the TUI dynamically displays the current working directory and the active LLM model.
*   **Improved Rendering**: The conversation and input boxes use rounded borders and internal padding for a cleaner look and to improve the native mouse selection experience.
*   **Character-Level Text Selection**: Select individual characters with pixel-perfect precision. Selections span multiple messages seamlessly.

## Architecture

The codebase is organized into focused, modular components:

### Core Modules

- **`main.rs` (99 lines)**: Event loop orchestrator. Handles terminal setup, renders UI, polls for events, and manages the main application loop.
- **`handlers.rs` (461 lines)**: Centralized event dispatcher. Processes all keyboard and mouse events, coordinates state changes, and delegates to specialized handlers.
- **`renderer.rs` (301 lines)**: UI rendering engine. Manages frame rendering for Chat, Settings, and Help modes using Ratatui.
- **`app.rs` (108 lines)**: Application state management. Defines the App struct containing chat history, models, configuration, and UI state.
- **`llm.rs` (202 lines)**: LLM integration. Handles communication with Ollama, streaming responses, and model fetching.
- **`mcp.rs` (195 lines)**: Model Context Protocol client. Manages MCP server communication for tool-use functionality.
- **`clipboard.rs` (65 lines)**: Multi-platform clipboard operations. Supports macOS (pbcopy), X11 (xclip/xsel), Wayland (wl-copy), and Windows (clip).
- **`mouse.rs` (60 lines)**: Mouse event utilities. Converts screen coordinates to text positions for selection.
- **`ui.rs` (80 lines)**: UI types and constants. Defines AppMode, Focus enums, and help/ASCII art content.
- **`config.rs` (57 lines)**: Configuration management. Handles loading and saving Ollama URL and model preferences.
- **`context.rs` (104 lines)**: LUCIUS.md context engine. Finds and loads context files from the project hierarchy.

### Design Principles

1. **Single Responsibility**: Each module has one clear purpose.
2. **Event-Driven**: All user interactions flow through the centralized `handle_events()` dispatcher.
3. **Non-Blocking I/O**: Clipboard operations and LLM requests use background processes/tasks to prevent UI freezing.
4. **Modular State**: App state is cleanly organized and passed through handler functions.

## Requirements

*   **Rust Toolchain**: This project requires a Rust toolchain version `1.70.0` or newer. It is recommended to use `rustup` to manage your Rust installations:
    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    rustup update
    ```
    You can check your Rust version with `rustc --version`.

*   **Ollama**: You need a running Ollama instance to serve local LLMs. Install from [ollama.ai](https://ollama.ai).

*   **Clipboard Tools** (optional, for system clipboard integration):
    - **Linux/X11**: `xclip` or `xsel`
    - **Linux/Wayland**: `wl-copy`
    - **macOS**: `pbcopy` (included)
    - **Windows**: `clip` (included)

## Building & Running

### Debug Build

```bash
cargo build
cargo run --bin lucius
```

### Release Build (Optimized)

```bash
cargo build --release
./target/release/lucius
```

### Running from Anywhere

Copy the executable to your PATH:

```bash
sudo cp target/release/lucius /usr/local/bin/
```

Then run from any directory:

```bash
lucius
```

### Development Workflow

1. **Running with hot-reload** (requires `cargo-watch`):
   ```bash
   cargo install cargo-watch
   cargo watch -x 'run --bin lucius'
   ```

2. **Running tests**:
   ```bash
   cargo test
   ```

3. **Building documentation**:
   ```bash
   cargo doc --open
   ```

## Configuration

Lucius stores its configuration in:
- **Linux/macOS**: `~/.config/lucius/config.toml`
- **Windows**: `%APPDATA%\lucius\config.toml`

The configuration file stores:
- Ollama URL (default: `http://192.168.1.42:11434`)
- Selected model name

You can manually edit this file or use the Settings screen in the app (`Ctrl+S`).

## Troubleshooting

### Common Issues

**Q: "Connection refused" when starting Lucius**
- Ensure Ollama is running: `ollama serve`
- Check the configured Ollama URL in Settings (`Ctrl+S`)
- Default URL: `http://192.168.1.42:11434`

**Q: Mouse selection not working**
- Ensure your terminal supports mouse events (most modern terminals do)
- Try a different terminal emulator if issues persist

**Q: Clipboard operations failing**
- Install the appropriate clipboard tool for your system (see Requirements)
- Run in a terminal that supports clipboard operations (not all SSH sessions support this)

**Q: "No models available"**
- Ensure Ollama is running with at least one model: `ollama pull llama2`
- Check connection by running `Ctrl+R` in Settings mode

### Debugging

Enable debug logging:

```bash
RUST_LOG=debug cargo run --bin lucius
```

Logs are written to `lucius.log` in the current directory.
