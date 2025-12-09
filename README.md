# Lucius CLI (Rust)

A blazing fast, lightweight (sub-20MB RAM) TUI for local LLMs, written in Rust.

## Features



-   **Lightweight & Fast**: Built with Rust and `ratatui` for minimal resource usage and a responsive feel.

-   **Context Engine (`LUCIUS.md`)**: Automatically finds and uses a `LUCIUS.md` file in your project's directory hierarchy to provide persistent system-level context to the LLM.

-   **Distributed Homelab Management (MCP)**: Lucius acts as a central control plane. It enables the LLM to interact with remote worker agents (built using `mcp-worker`) deployed across your homelab.

    -   **Tool-use Loop**: The LLM generates command tasks, which Lucius pushes to a Redis queue. Remote `mcp-worker` agents pick up these tasks, execute them, and report results back to Redis.

-   **Clipboard Integration**: Easily copy the last response from Lucius to the system clipboard using `Ctrl+Y`.

-   **Model Management**: Switch between different local models, see connection status, and refresh the model list from within the UI.

-   **Persistent Configuration**: Remembers your Ollama URL and selected model between sessions.

-   **Modern UI**: A clean interface with rounded borders, padded text, and dynamic information display.

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

## Running the Executables

Lucius is comprised of two main components:
1.  The `lucius` TUI application (your primary interface).
2.  The `lucius-mcp-worker` (a background agent that runs on remote machines, managed as a Git Submodule).

### Two-Command Install & Build

To get started with both the `lucius` client and the `lucius-mcp-worker` agent, use the following two commands from your terminal:

1.  **Clone the Main Repository Recursively:**
    ```bash
    git clone --recursive https://github.com/rsmedrano-cloud/lucius-tui.git
    cd lucius-tui
    ```
    *Note: The `--recursive` flag is crucial as it also clones the `lucius-mcp-worker` submodule. If you cloned without it, run `git submodule update --init --recursive` from the `lucius-tui` directory.*

2.  **Build Both Components (Workspace Build):**
    All subsequent commands should be run from the root of the `lucius-tui` directory.
    ```bash
    cargo build --release --workspace
    ```
    *This command builds both the `lucius` TUI client and the `lucius-mcp-worker` agent. The executables will be placed in the `target/release` directory at the root of the project.*

### Running the `lucius` TUI Application

After building, run the main `lucius` application from the project root:
```bash
./target/release/lucius
```
Alternatively, you can use `cargo run`:
```bash
cargo run --release --bin lucius
```

### Running the `lucius-mcp-worker` Agent

The `lucius-mcp-worker` runs on a target machine in your homelab.

1.  **Build the worker:** If you haven't already, run the workspace build command from the project root:
    ```bash
    cargo build --release --workspace
    ```
2.  **Set Environment Variables**: The worker needs to know the address of your Redis server.
    ```bash
    export REDIS_HOST="192.168.1.93" # Replace with your Redis IP/hostname
    ```
3.  **Run the worker**: From the project root, run the worker in the background:
    ```bash
    ./target/release/lucius-mcp-worker > mcp-worker.log 2>&1 &
    ```

### Making `lucius` Globally Accessible

To run `lucius` from any directory, you can copy the executable to a directory in your system's `PATH`.

```bash
# Ensure you are in the lucius-tui project root
sudo cp target/release/lucius /usr/local/bin/
```
Now, you should be able to simply type `lucius` in your terminal to start the application.

