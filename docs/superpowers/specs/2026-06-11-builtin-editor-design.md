<!-- Purpose: Captures the built-in editor design spec used for implementation work. -->

# 🏛️ Built-in Editor (F4) — Design

> Status: implemented. This file is the original design record. Current editor behavior is in the code and in `docs/editor-and-replay.md` and `docs/editor-lsp.md`.

**Date:** 2026-06-11
**Problem:** On Linux, F4 launches VS Code (or `$EDITOR`/nano) as an external
process. Far Manager opens its own internal editor on F4; FFM should do the
same.

## ⚖️ Decision

F4 opens a new built-in editor overlay (`Overlay::Editor`). The existing
external-editor integration stays available on **Alt+F4**, matching Far
Manager's binding for "edit with external editor".

### ⚖️ Approaches considered

1. **Built-in editor overlay (chosen).** True Far Manager parity; reuses the
   existing Overlay/popup/render architecture (same pattern as the F3 Viewer).
2. Drop only the VS Code preference, keep `$EDITOR`/nano. Rejected: nano is
   still an external editor, not "встроенный редактор как в Far Manager".
3. Full-featured editor (undo, search, syntax highlighting). Rejected: YAGNI.

## 🧱 Components

- **`models.rs` — `Editor` struct + `Overlay::Editor(Editor)` variant.**
  Fields: `title`, `path`, `lines: Vec<String>`, `cursor_line`,
  `cursor_col` (in *chars*, not bytes), `modified`, `confirm_exit`,
  `trailing_newline`. Methods: cursor movement (clamping), `insert_char`,
  `insert_newline`, `backspace`, `delete_forward`, `content()` (join + restore
  trailing newline), `save()` (`fs::write`, clears `modified`).
- **`editor.rs` — `handle_edit_builtin(app)`.** Guards (no selection / directory
  / >10 MB / non-UTF-8 → status message, never an error), loads the file, sets
  the overlay. External `handle_edit` is unchanged.
- **`action.rs`** — `Action::Edit` (F4) now means built-in editor; new
  `Action::EditExternal` for Alt+F4 (checked before bare F4, like Shift+F4).
- **`popup.rs` — `process_editor_input`.** Typing/Enter/Backspace/Delete edit;
  arrows/Home/End/PgUp/PgDn navigate; Tab inserts 4 spaces; **F2** saves;
  **Esc/F10** closes — if modified, switches to a confirm state
  (`Y`/`Enter` save+close, `N` discard+close, `Esc` continue editing).
  Panels are refreshed after save/close.
- **`ui.rs`** — renders the overlay like the Viewer (90×90 centered, FAR_BLUE),
  plus terminal cursor via `Frame::set_cursor`. Scroll is derived statelessly
  from the cursor (cursor kept ~centered vertically, visible horizontally) —
  no scroll state to keep in sync. The confirm state renders a small dialog.

## 🪶 Known simplifications (KISS)

- Cursor column counts chars; wide glyphs (CJK/emoji) may misalign the visual
  cursor by a cell. Acceptable for v1.
- CRLF files are saved with LF endings (Rust `str::lines()` semantics).
- No undo; vertical moves clamp the column (nano behaviour).

## 🚨 Error handling

File-level failures (binary file, too large, save error) are reported via
`status_message`; `io::Result` is reserved for panel-reload failures, matching
`handle_view`/`handle_edit_impl`.

## 🧪 Testing

Unit tests for `Editor` editing ops (incl. Unicode and trailing-newline
round-trip), `handle_edit_builtin` guards, popup key handling (save/confirm
flows write real files via `tempfile`), `resolve()` for Alt+F4, and a render
smoke test. Existing F4 event-loop tests updated: F4 now opens the overlay
instead of running the external-editor stub.
