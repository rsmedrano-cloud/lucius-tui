# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2025-12-08

### Fixed

- **`mcp-worker` Instability**: Resolved critical issues causing the `mcp-worker` to crash due to unhandled Redis connection failures, `LPOP` errors, and invalid JSON task payloads.
    - Implemented comprehensive "panic-proof" error handling using `match` statements for all Redis operations and JSON deserialization.
    - Ensured continuous operation of the worker, gracefully logging errors and continuing to process the task queue.

### Changed

- **`mcp-worker` Architecture Clarified**: Replaced the previous understanding of `mcp-worker` with a fully asynchronous, Redis-queue-based, and robust task processing agent. The `shell-mcp` is now understood as part of an older, deprecated architecture.
- **Improved Logging**: Integrated a robust logging mechanism within `mcp-worker` to provide clear operational feedback and error diagnostics.

### Added

- **Architectural Recommendation**: Documented the recommendation to transition `mcp-worker` into a separate repository (`lucius-mcp-worker`) to better reflect its role as an independent server agent and streamline deployment.

## [0.3.0] - 2025-12-06

### Added

- **Tool-use Loop**: Implemented a robust LLM-tool interaction loop, allowing the LLM to generate tool calls, execute them (via MCP), and receive results to inform its responses.
- **`shell-mcp` Server**: Created a custom MCP server binary (`shell-mcp`) for executing shell commands.
    - `exec` tool: Executes local shell commands.
    - `remote_exec` tool: Executes non-interactive commands on remote hosts via SSH.
- **Updated `LUCIUS.md` Tool Instructions**: The `LUCIUS.md` system prompt now includes detailed instructions for the LLM on how to use the `exec` and `remote_exec` tools.

### Changed

- **Version Bump**: Project version updated to `0.3.0` to reflect the addition of the new tool-use framework.

## [0.2.0] - 2025-12-06

### Added

- **Yank to Clipboard**: Press `Ctrl+Y` to copy the last response from Lucius to the system clipboard.
- **Status Message**: The status line now provides feedback for actions, such as confirming that content has been copied.
- **Robust Clipboard on Linux**: The clipboard feature now uses the `wl-copy` command-line tool on Wayland systems to ensure reliable copying.

### Fixed

- **Clipboard Implementation**: Replaced a buggy clipboard library (`arboard`) with a more robust, external command-based approach (`wl-copy`), fixing an issue where content would not be available for pasting.
- **Mouse Selection of Borders**: Mitigated the issue where terminal-native mouse selection would include border characters by adding internal padding to the conversation view.
- **Incorrect Keybinding**: Fixed the `Ctrl+Y` keybinding to correctly trigger the yank action.

### Changed

- **UI Appearance**: The conversation and input boxes now have rounded borders and internal padding for a cleaner, more modern look.
- **Version Bump**: Project version updated to `0.2.0` to reflect the addition of new features and fixes.

## [0.1.2] - 2025-12-05

- Initial release with core TUI, Ollama connection, and `LUCIUS.md` context engine.
