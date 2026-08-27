<!-- Purpose: Introduces FunnyFileManager and the main setup, build, and usage paths. -->

# FunnyFileManager (ffm)
Far Manager interface implementation in Rust

## Description

**ffm** is a high-performance, terminal-based file manager with a dual-panel interface inspired by classic tools like Far Manager and Norton Commander. Built with Rust and the `ratatui` TUI library, it provides a fast, keyboard-driven experience for file system navigation and management.

## Features

- **Dual Panel Interface**: Classic side-by-side view for efficient file operations.
- **Fast Navigation**: Instant directory browsing with keyboard shortcuts.
- **File Operations**: Full support for Copy, Move, Rename, Mkdir, and Delete.
- **S3 Integration**: Browse AWS S3 buckets as if they were local drives (Alt+F1/F2).
- **Quick Search**: Type to jump to files instantly (Alt+Letter).
- **Bookmarks**: Save and jump to frequently used directories.
- **Cross-Platform**: Seamless experience on Linux, macOS, and Windows.

## Installation

### As a System Utility

To install `ffm` as a system-wide utility (adds to your PATH):

```bash
cargo install --path .
```

After installation, you can run it from anywhere using:

```bash
ffm
```

### From Source

```bash
git clone https://github.com/stden/FunnyFileManager.git
cd FunnyFileManager
cargo build --release
./target/release/ffm
```

## Usage

### AI Agent Helpers

On Windows, you can launch repository-aware agent runs with:

```bat
claude-auto.cmd
codex-auto.cmd
gemini-auto.cmd
```

Each script changes into the repository root and sends a default prompt asking the agent to implement missing required functionality and verify it with Cargo. You can also pass your own prompt:

```bat
codex-auto.cmd Fix the S3 panel bugs and add tests for the new behavior.
```

### Complete Hotkey Guide

| Key | Action |
|-----|--------|
| **↑/↓** | Navigate up/down in active panel |
| **Enter** | Open selected directory or file |
| **Backspace** | Go to parent directory |
| **Tab** | Switch between left and right panels |
| **Insert** or **Space** | Mark/unmark file and move down |
| **F1** | Toggle Help window |
| **F3** | View file content (pager) |
| **F4** | Edit file in the built-in editor |
| **Alt+F4** | Edit file with VS Code, `$EDITOR`, or `nano` |
| **Shift+F4** | Create new file |
| **F5** | Copy marked or selected files to the other panel |
| **F6** | Move/Rename marked or selected files to the other panel |
| **F7** | Create new directory |
| **F8** | Delete marked or selected files |
| **F10** | Quit application (or type `quit`) |
| **Alt+F1** | Select drive/S3 for Left panel |
| **Alt+F2** | Select drive/S3 for Right panel |
| **Ctrl+U** | Swap Left and Right panel paths |
| **Ctrl+O** | Toggle console mode (hide/show panels) |
| **Ctrl+R** | Refresh current directory |
| **Ctrl+\\** | Jump to root directory |
| **Alt + Letter** | Quick search: jump to file starting with letter |
| **:** | Open Command Line mode |

### Command Line Mode

Press `:` to enter command mode. Supported commands:

- `cd <path>`: Change directory in active panel
- `pwd`: Show the active directory
- `sort <name|size|ext|extension>`: Change panel sorting
- `bookmark set <0-9>` / `bookmark go <0-9>`: Manage bookmarks
- `mark all`: Mark all visible entries
- `unmark`: Clear all marks
- `help`, `mcp`, `winmsg`, `winperf`: Open built-in reports or help
- `quit` or `exit`: Close ffm

Unknown commands fall back to the platform shell in the active panel directory.

## Architecture

The project is modularly designed:

- **`models`**: Core data structures (`App`, `Panel`, `DirEntry`).
- **`action`**: Pure main-view key-to-action mapping.
- **`popup`**: Strictly modal input handling; active overlays consume keys before action resolution.
- **`command`**: Typed command parsing.
- **`ops/`**: Canonical command, file, shell, viewer, and MCP-status operations.
- **`handlers/mod.rs`**: Backward-compatible re-export facade.
- **`terminal`**: RAII lifecycle for the TUI and external-editor suspension.
- **`ui`**: Stateless rendering logic using `ratatui`.
- **`s3`**: Virtual drive abstraction for AWS S3.

## Development

```bash
cargo test                         # Run unit and integration tests
cargo tarpaulin                    # Check code coverage
cargo mutants                      # Run mutation testing
cargo bench --bench utils_bench    # Run Criterion benchmarks for basic helpers/models
```

Additional repository guidance lives in [docs/best-practices.md](docs/best-practices.md), including testing conventions and the preferred full-suite command with `--features test-helpers`.

## Testing

- Repository-wide testing/TDD guide: `TESTING_TDD_GUIDE.md`
- Stack checklist: `TESTING_CHECKLIST_RUST.md`

## License

See [LICENSE](LICENSE) file for details.
