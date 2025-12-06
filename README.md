# Lucius CLI (Rust)

A blazing fast, lightweight (sub-20MB RAM) TUI for local LLMs, written in Rust.

## Features

- **Lightweight & Fast**: Built with Rust and `ratatui` for minimal resource usage and a responsive feel.
- **Context Engine (`LUCIUS.md`)**: Automatically finds and uses a `LUCIUS.md` file in your project's directory hierarchy to provide persistent system-level context to the LLM.
- **Tool-use Loop**: Enables the LLM to interact with external tools (MCP servers) by generating tool calls, executing them, and incorporating the results into its responses.
- **Clipboard Integration**: Easily copy the last response from Lucius to the system clipboard using `Ctrl+Y`.
- **Model Management**: Switch between different local models, see connection status, and refresh the model list from within the UI.
- **Persistent Configuration**: Remembers your Ollama URL and selected model between sessions.
- **Modern UI**: A clean interface with rounded borders, padded text, and dynamic information display.

## Keybindings

| Key                 | Action                                       |
| ------------------- | -------------------------------------------- |
| `Ctrl+H`            | Toggle the help screen.                      |
| `Ctrl+S`            | Switch to the Settings screen.               |
| `Ctrl+Q`            | Quit the application.                        |
| `Ctrl+L`            | Clear the chat history.                      |
| `Ctrl+Y`            | Yank (copy) the last response to the clipboard. |
| `Ctrl+T`            | In Chat mode, list available MCP tools in the status bar. |
| `Esc`               | Exit modal screens (Help/Settings) or interrupt a streaming response. |
| `Enter`             | Send the message in the input box.           |
| `Tab`               | In Settings, switch focus between inputs.    |
| Mouse Scroll        | Scroll the conversation history.             |
| `Shift` + Mouse Drag | Select text using the terminal's native selection. |


## UI/UX Enhancements

*   **Dynamic Status Line**: A status line is displayed between the conversation and input box. It shows whether a `LUCIUS.md` file is in use, provides feedback for actions like copying to the clipboard, and indicates the status of the MCP server.
*   **Bottom Bar Information**: The bottom of the TUI dynamically displays the current working directory and the active LLM model.
*   **Improved Rendering**: The conversation and input boxes use rounded borders and internal padding for a cleaner look and to improve the native mouse selection experience.

## Requirements

*   **Rust Toolchain**: This project requires a Rust toolchain version `1.70.0` or newer. It is recommended to use `rustup` to manage your Rust installations:
    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    rustup update
    ```
    You can check your Rust version with `rustc --version`.

## Running the Executable

After building the project in release mode, you can find the executables at `target/release/lucius` and `target/release/shell-mcp`.

To build both executables:
```bash
cargo build --release
```

To run the main application:

```bash
./target/release/lucius
```

Alternatively, you can use `cargo run` if you specify the binary:
```bash
cargo run --bin lucius
```

### Making it Globally Accessible

To run `lucius` from any directory by simply typing `lucius`, you need to add its location to your system's `PATH`.

**Option 1: Copy to a PATH directory (Recommended for convenience)**

```bash
sudo cp target/release/lucius /usr/local/bin/
```

After this, you can just type `lucius` in your terminal.

**Option 2: Add to your shell's PATH (Temporary or Permanent)**

*   **Temporarily (for current session):**
    ```bash
    export PATH="/path/to/your/target/release:$PATH"
    ```

*   **Permanently (add to shell configuration):**
    Edit your shell's configuration file (e.g., `~/.bashrc`, `~/.zshrc`, `~/.profile`) and add the `export PATH="..."` line. Then, `source` the file (e.g., `source ~/.bashrc`) or restart your terminal.

    Example for `~/.bashrc`:
    ```bash
    echo 'export PATH="/path/to/your/target/release:$PATH"' >> ~/.bashrc
    source ~/.bashrc
    ```

**Note:** If you are running `lucius` from inside the `lucius` project directory, you can also use `cargo run --release`.
