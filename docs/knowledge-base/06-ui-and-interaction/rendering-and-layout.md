<!-- Purpose: Documents ratatui rendering, layout, and visual conventions. -->

# Rendering and Layout

The UI is entirely built with `ratatui` (version 0.26 at time of writing).

## Layout Structure (from top to bottom)

1. Header (1 line) — project title / current info
2. Two panels (main area, 50/50 split)
3. Status bar (2 lines) — file info + hints
4. Command line (1 line) — when active
5. F-key command bar (1-2 lines)

The layout is defined in `src/ui.rs` using `ratatui::layout`.

## Color Scheme

- Background: `FAR_BLUE` = `RGB(0, 0, 128)` (classic Far look)
- Active border: Cyan + Bold
- Inactive border: Blue
- Directories: White + Bold
- Files: Cyan
- Selection: Black on Cyan background

These colors are defined as constants in `ui.rs`.

## Panel Rendering

Each panel (`render_panel`) renders:
- Border with current path as title
- List of `DirEntry` items
- Icons (📁 / 📄) and size formatting via `format_size`
- Highlighting for marked items and current selection

## Overlays

All overlays are drawn on top after the main panels (see `render_overlays`).

They use `centered_rect` helper for modal positioning.

Current overlays include Help, Viewer, Drive selection, Delete confirmation, New file input, etc.

`Overlay::Viewer` is reserved for content the user asked to see (F3, `:mcp`, the
Windows diagnostics). Ordinary shell output does not pop a window — it goes to
the console layer below.

## Console Layer and Console Mode (Ctrl+O)

Far Manager keeps command output on the console screen and draws the panels over
it. FFM runs on ratatui's alternate screen, which has no such screen, so the
transcript is kept in `ConsoleLog` (`src/console.rs`) and painted as the
**bottom layer of every frame**:

1. `render_console_backdrop` — black ground over the whole area, always.
2. `render_console_text` — one transcript row per line, filling downwards from
   the top like a real console. Command echoes are White + Bold, stdout Gray,
   stderr LightRed.
3. Header, panels, status bar, command line and F-key bar are drawn on top,
   covering it completely in normal operation.

`Ctrl+O` sets `App.view` to `ViewMode::Console` (**not** an overlay — see
below), and `ui()` returns after step 2, so the panels are never drawn and the
transcript stays on screen. The bottom row belongs to the command line when the
prompt is open and to the hint otherwise; the transcript is drawn into the area
*minus* that row, because writing into the full area first would hide the newest
line under it — the one line the user pressed Ctrl+O to read.

**`ViewMode` is orthogonal to `Overlay`.** `Overlay` says *which dialog is
open*; `ViewMode` says *what is drawn behind it*. They were one enum until
`Overlay::Console` was removed, and that made them mutually exclusive: the
command line could not be used while the console showed, which is the entire
point of Far Manager's Ctrl+O. Now typing opens the prompt without leaving the
console, and each command's output appears directly above it.

Console-mode keys go through the main-view `Action` path, not `popup`
(`lib::console_action`): `Up`/`Down`, `PgUp`/`PgDn` and `Home`/`End` scroll the
transcript — a page is the whole terminal, since the console is full-screen —
`Ctrl+O` puts the panels back, F10 still quits, and the command-line actions
pass through. Everything else is swallowed: the panels are off screen, and F8
must not leave a delete dialog behind a full-screen console.

Lines are **cut, never wrapped**: one transcript line per row is what keeps
`ConsoleLog`'s scroll arithmetic honest, since a wrapped line would occupy rows
the log does not know it spent. New output snaps the view back to the bottom,
the way a terminal does. The ring is capped at `CONSOLE_CAP` (5000) lines and
lives in memory only — unlike `history`, nothing here reaches disk.

## Performance Considerations

- Rendering is done every frame in the event loop.
- No heavy computation should happen inside render functions.
- Text is built as `Vec<Line>` / `Vec<Span>` for efficient ratatui rendering.

See `src/ui.rs` for the full implementation.