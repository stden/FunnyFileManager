<!-- Purpose: Explains the Overlay state machine and modal behavior. -->

# Overlays and Modals

One of the most important architectural features of FunnyFileManager is the centralized handling of modal overlays.

## The Overlay Enum

Instead of many independent `bool` and `Option<T>` fields (`show_help`, `viewer`, `command_line`, `show_drives`, `delete_confirm`, `new_file_input`, `show_console`, etc.), the project uses a single canonical enum:

```rust
pub enum Overlay {
    None,
    Help,
    Viewer(Viewer),
    Editor(Editor),
    DriveSelection { side: PanelSide, selected_index: usize },
    NewFileInput(String),
    CommandLine(CommandLine),
    DeleteConfirm(Vec<PathBuf>),
    Console,
}
```

This is stored in `App.overlay`.

`CommandLine` owns both the editable text buffer and the fuzzy-completion menu
state. Keeping them inside the overlay makes menu visibility and selection part
of the same mutually exclusive modal state.

## Benefits

- Mutual exclusivity is enforced by the type system (only one overlay can be active).
- Pattern matching on the overlay is exhaustive and clear.
- Opening/closing logic is centralized via `app.set_overlay(...)` and `app.close_overlay()`.

## Current Overlay Types

- **Viewer** (F3) — also reused for MCP status reports.
- **Editor** (F4) — built-in text editor.
- **Help** (F1)
- **CommandLine** (`:`) — modal buffer plus fuzzy command/path dropdown
- **DriveSelection** (Alt+F1 / Alt+F2)
- **DeleteConfirm** (F8)
- **NewFileInput** (Shift+F4)
- **Console** (Ctrl+O — special full-screen mode that hides the panels)

## Implementation Locations

- State definition: `src/models.rs`
- Input handling: `src/popup.rs` → `process_popup_input` (re-exported via
  `src/handlers/mod.rs` for compatibility)
- Rendering: `src/ui/overlays.rs` → `render_overlays`
- Opening logic: `src/lib.rs` (main-view actions) and the various `handle_*` /
  `set_overlay` call sites in `editor` and `ops`

Every variant must be reachable from a key binding or a `:` command and have an
intentional presentation in `src/ui.rs` or `src/ui/overlays.rs`. A variant that
is only constructed in tests is dead weight: it makes `process_popup_input`
swallow every key while nothing is painted. There was once a `Search(String)`
and a `Bookmarks` variant in exactly that state; both were deleted. Incremental
search is `Alt+letter` (quick search); bookmarks are
`:bookmark set|go <0-9>`.

While an overlay is active, every key is routed to `process_popup_input` before
main-view action resolution. Unsupported keys remain consumed; they cannot fall
through to panel operations.

## Compatibility Accessors

Helpers such as `get_viewer`, `get_command_line`, and
`get_delete_confirm` project data from the active enum variant. They do not
create parallel modal fields; new modal state still belongs in `Overlay`.
