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

To get started with both the `lucius` client and the `lucius-mcp-worker` agent, use the following two commands:

1.  **Clone the Main Repository Recursively:**
    ```bash
    git clone --recursive https://github.com/rsmedrano-cloud/lucius-tui.git # Replace with your actual TUI repo URL
    cd lucius-tui # Or whatever you named the cloned directory
    ```
    *Note: The `--recursive` flag is crucial. It automatically clones the `lucius-mcp-worker` submodule, which contains the worker's code. If you cloned without `--recursive`, run `git submodule update --init --recursive` from the `lucius-tui` directory.*

2.  **Build Both Components (Workspace Build):**
    ```bash
    cargo build --release --workspace
    ```
    *This command will build both the `lucius` TUI client (from the `lucius` sub-directory) and the `lucius-mcp-worker` agent (from the `lucius-mcp-worker` sub-directory).*
    *Executables will be found at `target/release/lucius` (for the client) and `target/release/lucius-mcp-worker` (for the worker), relative to the root of this repository.*

### Running the `lucius` TUI Application

To run the main `lucius` application:
```bash
./target/release/lucius
```
Alternatively, you can use `cargo run` from the `lucius` directory:
```bash
cd lucius && cargo run --bin lucius
```

### Running the `lucius-mcp-worker` Agent

The `lucius-mcp-worker` is designed to run on a target machine in your homelab. It connects to a central Redis instance to receive tasks.

1.  **Build (if not done with workspace build):**
    If you only built the `lucius` client, navigate to the `lucius-mcp-worker` directory and build it:
    ```bash
    cd lucius-mcp-worker && cargo build --release
    ```
2.  **Environment Setup**: Ensure `REDIS_HOST` is set in your environment or a `.env` file where the worker is running.
    ```bash
    export REDIS_HOST="192.168.1.93" # Replace with your Redis IP/hostname
    ```
    Or, create a `.env` file in the worker's directory:
    ```
    REDIS_HOST=192.168.1.93
    ```

3.  **Run in Background**: To run the `lucius-mcp-worker` continuously in the background and log its output:
    ```bash
    ./target/release/lucius-mcp-worker > lucius-mcp-worker.log 2>&1 &
    ```
    This command redirects `stdout` and `stderr` to `lucius-mcp-worker.log` and runs the process in the background.

### Making `lucius` Globally Accessible

To run `lucius` from any directory by simply typing `lucius`, you need to add its location to your system's `PATH`.

**Option 1: Copy to a PATH directory (Recommended for convenience)**

```bash
sudo cp target/release/lucius /usr/local/bin/
```
After this, you can just type `lucius` in your terminal.

**Option 2: Add to your shell's PATH (Temporary or Permanent)**

*   **Temporarily (for current session):**
    ```bash
    export PATH="/path/to/your/lucius/target/release:$PATH"
    ```

*   **Permanently (add to shell configuration):**
    Edit your shell's configuration file (e.g., `~/.bashrc`, `~/.zshrc`, `~/.profile`) and add the `export PATH="..."` line. Then, `source` the file (e.g., `source ~/.bashrc`) or restart your terminal.

    Example for `~/.bashrc`:
    ```bash
    echo 'export PATH="/path/to/your/lucius-tui/target/release:$PATH"' >> ~/.bashrc
    source ~/.bashrc
    ```

