# Lucius CLI (Rust)

A blazing fast, lightweight (sub-20MB RAM) TUI for local LLMs, written in Rust.

## Context Engine (LUCIUS.md)

Lucius can load a `LUCIUS.md` file to provide system-level context to the LLM.
When the application starts, it traverses parent directories from its current working directory, searching for a file named `LUCIUS.md`. If found, its content is automatically loaded and sent as a system message with each chat request to the LLM. This allows you to "prime" the model with specific instructions, persona, or common information relevant to your project or task.

## User Interface Enhancements

*   **Unified Help Screen (`Ctrl+H`)**: Replaced the previous long help message with a dedicated, toggleable help screen. Press `Ctrl+H` to view a comprehensive list of all shortcuts and their descriptions.
*   **Consistent Exit Key (`Esc`)**: The `Esc` key now consistently exits all modal screens (Help, Settings) and returns to the chat interface. Press `Esc` to exit the Help screen or Settings mode. Press `Ctrl+H` again also exits the help screen.
*   **Dynamic Status Line**: A new status line is displayed between the conversation and input box in chat mode. It currently shows:
    *   Whether a `LUCIUS.md` context file is being used.
    *   A placeholder for the number of active MCP servers (future feature).
*   **Bottom Bar Information**: The very bottom line of the TUI now dynamically displays:
    *   The current working directory (bottom-left).
    *   The currently active model (bottom-right).

## Requirements

*   **Rust Toolchain**: This project requires a Rust toolchain version `1.70.0` or newer. It is recommended to use `rustup` to manage your Rust installations:
    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    rustup update
    ```
    You can check your Rust version with `rustc --version`.

## Running the Executable

After building the project in release mode, you can find the executable at `target/release/lucius`.

To run the application directly:

```bash
./target/release/lucius
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
