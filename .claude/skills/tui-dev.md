<!-- Purpose: Documents the Claude skill for terminal UI development tasks. -->

---
name: tui-dev
description: Use when working on the TUI rendering, layout, colors, panels, overlays, or keyboard handling in FFM.
---

# TUI Development

## Overview
FFM uses ratatui + crossterm for terminal UI. Far Manager-inspired dark blue theme.

## Key Files
- `src/ui.rs` — All rendering (panels, status bar, overlays)
- `src/handlers.rs` — Keyboard event handling
- `src/models.rs` — App/Panel state structs
- `src/lib.rs` — Event loop (`run_app_with_event_reader`)

## Color Scheme
```rust
FAR_BLUE = RGB(0, 0, 128)  // Background
Cyan + Bold                  // Active border
Blue                         // Inactive border
White + Bold                 // Directories
Cyan                         // Files
Black on Cyan                // Selection highlight
```

## Layout
```
+------------------------------------------+
| Header (1 line)                          |
+-------------------+----------------------+
| Left Panel (50%)  | Right Panel (50%)    |
+-------------------+----------------------+
| Status Bar (2 lines)                     |
+------------------------------------------+
| Command Bar (2 lines)                    |
+------------------------------------------+
```

## Testing UI
```bash
# Run UI tests
cargo test ui_tests -q

# Run with WritableTestBackend (no real terminal)
cargo test --features test-helpers -q

# Visual check
cargo run
```

## Overlays (mutually exclusive)
Help (F1), Viewer (F3), Drive selector (Alt+F1/F2), Delete confirm (F8), New file (Shift+F4), Console (Ctrl+O)
