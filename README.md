# Lucius CLI (Rust)

A blazing fast, lightweight (sub-20MB RAM) TUI for local LLMs, written in Rust.

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
    echo 'export PATH="/home/rodrigo/your/target/release:$PATH"' >> ~/.bashrc
    source ~/.bashrc
    ```

**Note:** If you are running `lucius` from inside the `lucius` project directory, you can also use `cargo run --release`.