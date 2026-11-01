<!-- Purpose: Documents ratatui rendering, layout, and visual conventions. -->

# 🏛️ Rendering and Layout

The UI is entirely built with `ratatui` (version 0.26 at time of writing).

## 🏛️ Layout Structure (from top to bottom)

1. Two panels (main area, 50/50 split)
2. Command line (1 line)
3. F-key command bar (1 line)

The key bar is `ui::fkey_bar::render_key_bar(f, area, labels)`. The panels pass
Far's `FKEY_LABELS`; a full-screen view with keys of its own passes its own
twelve `(key, label)` pairs. `labels` is an array of exactly `FKEY_COUNT`, the
count the hit test resolves clicks against, so a bar with another number of
cells does not compile. Either way the cells are sized by
`mouse::fkey_widths`, the arithmetic the click hit test `mouse::fkey_at` also
uses, so a click lands on the key whose label it hit. A bar of no keys has no
cells and answers no click.

Far spends no row above the panels and only two below them, and so does this.
There is no banner and no status bar: the banner repeated the F-key bar, and the
status bar's one real job — carrying messages — moved inside the panel, onto the
info row described below.

The command line keeps its cursor visible even before the first printable key
opens `Overlay::CommandLine`. It paints a reversed cell after the prompt and
places the real terminal cursor on that same cell; the painted backing therefore
remains visible during the terminal cursor's blink-off phase. Once editing
starts, both move together to the `LineEditor` caret.

The layout is defined in `src/ui.rs` using `ratatui::layout`.

## 🎨 Color Scheme

- Background: `FAR_BLUE` = `RGB(0, 0, 128)` (classic Far look)
- Frames: double-line box drawing for panels, overlays, and dialogs
- Active border: Cyan + Bold
- Inactive border: Blue
- Directories: White + Bold
- Files: Cyan
- Cursor: White on dark blue; a marked entry keeps its yellow foreground

Colors are named in `src/ui/palette.rs`. Forms use black on light gray with
cyan input fields and focused controls. Warnings use white on dark red;
their focused buttons use black on light gray. Vertical menus use white on
teal with black selection bars; Help uses black on teal. Functional hotkeys
use bright yellow on dark grounds and black underlined text on light or teal
grounds. RGB values keep the owned palette independent of ANSI terminal themes.
Panel theme adjustments run before menus and modal rendering; disabling file
highlighting preserves the cursor's foreground. The
[UI quality suite](../../ui-quality.md) guards text contrast and layout and
exports SVGs for visual review.

## 🎨 Panel Rendering

`render_panel` and its helpers live in `src/ui/panel.rs` and its `panel/`
children (columns, layout, rows, frame, info), each within the 200-line cap
`tests/file_line_limit_test.rs` enforces.

Inside the frame, top to bottom:

1. **Column header** — the sort-mode letter (`n`ame / `e`xtension / `s`ize) in
   the corner, then `Name`, `Size`, `Date`, `Time` centred over their columns.
2. **Entry rows** — the name column, then each remaining column behind a `│`.
3. **Divider** — the rule that closes the column separators. While entries are
   marked, Far's selection summary is written over its middle in the marked
   colour: `29 bytes in 2 files` (a marked folder is counted, not summed:
   `…, 1 folder`), shortened to `5.0 G in 2 files` where the exact count does
   not fit (`src/ui/panel/selection.rs`).
4. **Info row** — the current entry's full name, or the message that used to go
   to the status bar (quick search wins over both). A message too wide for
   the row gives up the head of the path it quotes first, down to the file
   name (`Cannot edit …/unreadable.txt: Permission denied`), and only then is
   cut at its end: `Status::path_message` keeps the path apart from the words.

The bottom border carries the panel totals and the filesystem's free space;
`src/diskfree.rs` wraps the platform call std has no equivalent for.

Listing and selection totals are cached by `models/panel/totals.rs`. Unchanged
frames read both totals in O(1); after an entry edit the file totals take O(n)
to rebuild, and selected totals take O(n log(m+1)) for m marked names.
`PanelCollection` invalidates the cache before mutable access, including edits
through `iter_mut` and indexing. Whole replacements use `entries.into()` or
assign through `*panel.entries`; reads and ordinary collection methods retain
their usual syntax. Clones and restored selections carry distinct observed
revisions as soon as their contents change.

### 📊 Columns

| Column | Width | Contents |
|--------|-------|----------|
| Name   | rest  | Base name left, extension right-aligned |
| Size   | 7     | `Up` for `..`, `Folder` for directories, `Link` for a link to a folder, `Broken` for a dangling link, else the byte count (a link to a file: its target's) |
| Date   | 8     | `MM/DD/YY`, local time |
| Time   | 5     | `HH:MM`, local time |

Rules worth knowing:

- A file's name splits at its **final** dot. Directories never split, and
  neither does a leading dot — `.env` is a hidden file, not an empty name.
- A byte count is printed exactly while it fits seven cells, and abbreviated
  Far-style (`381 K`, `49.4 G`) only when it does not.
- A cut name is marked with `}`, Far's marker, not an ellipsis.
- On a narrow panel the columns are dropped from the **right** — Time, then
  Date, then Size — so the panel loses a column rather than its file names.
- Only the active panel draws a cursor bar: two would give the screen two
  answers to "what does Enter act on". A marked entry under it keeps the
  marked yellow as the bar's text colour, so the bar never hides a mark.
- Hidden entries are dimmed — and only Windows has any: the
  `FILE_ATTRIBUTE_HIDDEN` bit. A leading dot on Unix is a naming convention,
  not an attribute, so `.cargo` is painted like any other directory, as Far
  paints it.

## 🪟 Overlays

All overlays are drawn on top after the main panels (see `render_overlays`).

They use `centered_rect` helper for modal positioning.

Current overlays include Help, Viewer, Drive selection, Delete confirmation, New file input, etc.

`Overlay::Viewer` is reserved for content the user asked to see (F3, `:mcp`, the
Windows diagnostics). Ordinary shell output does not pop a window — it goes to
the console layer below.

## 💻 Console Layer and Console Mode (Ctrl+O)

Far Manager keeps command output on the console screen and draws the panels over
it. FFM runs on ratatui's alternate screen, which has no such screen, so the
transcript is kept in `ConsoleLog` (`src/console.rs`) and painted by
`src/ui/console.rs` as the **bottom layer of every frame**:

1. `console::render_backdrop` — black ground over the whole area, always.
2. `console::render_text` — one transcript row per line, filling downwards from
   the top like a real console. Command echoes are White + Bold, stdout Gray,
   stderr LightRed. Returns the row count it filled.
3. Panels, status bar, command line and F-key bar are drawn on top,
   covering it completely in normal operation.

Programs that own the terminal (Enter on an executable, Update packages) are
not captured: they keep the real terminal so they can prompt and draw. Their
command line and how they ended go into the transcript; what they printed stays
on the terminal's own screen. After a program started with Enter, FFM therefore
prints `[exit status: N] Press any key to return to FFM` under its output and
brings the panels back on the next key, where Far would leave the output on its
console screen. Update packages keeps each step's stderr in the transcript too.

`Ctrl+O` sets `App.view` to `ViewMode::Console` (**not** an overlay — see
below) and `console::render_view` takes the frame, so the panels are never drawn
and the transcript stays on screen.

**The prompt follows the output.** `console::render_view` gives the transcript
every row but one, then puts the prompt on the row *directly after the newest
line* — the shape of every terminal: output grows downwards and the cursor
follows it. A transcript tall enough to fill the screen pushes the prompt onto
the last row, which is all scrolling means. The prompt is drawn whether or not
it is open, so the console always has a cursor waiting on it (typing is what
opens it), and nothing is pinned to the bottom of the screen: the earlier
bottom-pinned prompt left a band of dead black between the last line of output
and the place the next command is typed, which no console shows.

Scrolled back, the prompt has scrolled off the bottom exactly as it does in a
terminal, and `console::render_scroll_note` takes the row it vacated to say how
many lines are below. That is the one thing a scrollbar would show and this
console cannot; at the bottom, where there is nothing to say, it says nothing.

Because the console prompt can sit at the very top of the screen,
`completion_placement` puts the completion popup **below** the prompt when there
is no room above it. Above is still preferred — that is where Far draws it, and
in the panel view there is always room.

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

## ⚡ Performance Considerations

- Unchanged screens are not redrawn during idle polling.
- Queued file-panel navigation and wheel events share a frame: at most 64
  events or 8 ms of input processing before rendering. Reading ahead never
  waits; commands, resize events and errors remain queued until after the
  current frame. Every cursor movement still runs through normal dispatch.
- `application/frame.rs` surrounds each ratatui diff with synchronized-update
  markers, so supporting terminals present the complete frame together. The
  end marker is sent even if drawing fails. Other terminals retain normal
  ratatui diff rendering.
- No heavy computation should happen inside render functions.
- Text is built as `Vec<Line>` / `Vec<Span>` for efficient ratatui rendering.

See `src/ui.rs` for the full implementation.
