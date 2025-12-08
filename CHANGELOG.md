# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2025-12-07

### Added

- **Mouse-Driven Text Selection**: Click and drag to select text in conversation history with character-level precision.
  - Left-click + drag: Select text and auto-copy to clipboard on release
  - Middle-click + drag: Select text and copy to both clipboard and primary selection on release
  - Selection automatically highlighted with dark gray background
- **Advanced Text Selection UI**: Visual feedback showing selected text ranges across multiple messages
- **Comprehensive Developer Documentation**:
  - `CONTRIBUTING.md`: Complete guide for developers with setup, code organization, and contribution workflow
  - `ARCHITECTURE.md`: Deep technical documentation covering event flow, data structures, rendering, and design decisions
  - Enhanced `README.md` with architecture overview and improved keybindings table

### Changed

- **Major Architecture Refactor** (Code Quality Improvements):
  - Extracted all UI rendering to `renderer.rs::render_frame()` - single function handling all mode rendering
  - Extracted all event handling to `handlers.rs::handle_events()` - centralized event dispatcher
  - Reorganized mouse handlers: `handle_mouse_scroll_up/down`, `handle_left_mouse_down/drag/up`, `handle_middle_mouse_down/drag/up`
  - Reduced `main.rs` from 457 to 99 lines (-78%) - now contains only event loop and setup
  - Separated concerns: main orchestrates, handlers dispatch, renderer displays
- **Enhanced Keybindings**:
  - Added `Ctrl+R` to refresh models in Settings mode
  - Improved keybindings documentation with mouse actions
- **Improved README**: Added keybinding descriptions, troubleshooting, debugging section

### Fixed

- **Code Organization**: Eliminated "nesting hell" by centralizing event handling logic
- **Modularity**: Each module now has single, clear responsibility
- **Maintainability**: Reduced cognitive load for understanding event flow

### Performance

- No performance regression - event handling remains non-blocking
- Character-level selection uses efficient coordinate mapping
- Background tasks for clipboard operations prevent UI freezing

### Documentation

- Added `CONTRIBUTING.md` for developer onboarding
- Added `ARCHITECTURE.md` with detailed technical documentation
- Updated `README.md` with new features and architecture section
- Documented all modules and their responsibilities
- Included debugging and troubleshooting guides

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
