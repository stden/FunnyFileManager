# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/claude-code) when working with code in this repository.

## Project Overview

FunnyFileManager is a terminal-based dual-panel file manager inspired by Far Manager, written in Rust using the `ratatui` TUI library.

## Build Commands

```bash
cargo build          # Build the project
cargo run            # Run the file manager
cargo test           # Run tests
cargo tarpaulin      # Run tests with coverage
```

## Project Structure

```
src/
├── main.rs      # Entry point, event loop, keyboard handling
├── models.rs    # Data structures (Panel, App, DirEntry, Viewer, PanelSide)
├── ui.rs        # UI rendering (panels, header, status bar, overlays)
├── handlers.rs  # Event handlers (view, edit, copy, move, mkdir, delete)
└── utils.rs     # Utilities (format_size, copy_dir_all, unique_destination)

tests/
└── integration_test.rs  # Integration tests
```

## Key Architecture Decisions

- **Modular design**: Code is split into logical modules for maintainability
- **Far Manager color scheme**: Dark blue background (RGB 0,0,128), cyan text
- **Dual-panel layout**: Left and right panels with tab switching
- **F-key commands**: F1-Help, F3-View, F4-Edit, F5-Copy, F6-Move, F7-Mkdir, F8-Delete, F10-Quit

## Testing

Tests are in `src/main.rs` (unit tests) and `tests/integration_test.rs`. Use `tempfile::TempDir` for filesystem tests.

## Dependencies

- `ratatui` - TUI framework
- `crossterm` - Terminal handling
- `tempfile` - Temporary directories for testing
