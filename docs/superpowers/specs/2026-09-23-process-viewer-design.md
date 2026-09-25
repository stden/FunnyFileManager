<!-- Purpose: Design for the htop-style live process viewer behind Ctrl+W, F9 > Commands > Task list and the Change drive Process list row: Far-native keys with htop features, first-class on Linux. -->

# Task list (Ctrl+W): an htop-style live process viewer — design

_2026-09-23, rebased 2026-09-24. Status: approved design; T01-T20 are
implemented. Branch `feat/process-viewer-2`, in its own worktree, on main
`0125c57`. This file is the plan; nothing it describes exists yet unless it
says "today" or sections 16 to 18 say it landed._

Sources checked for this design, read-only:

- this repository at the first branch's base commit, `415db60`, and again at
  main `0125c57` for what section 16 lists;
- the htop 3.3.0 manual (`/usr/share/man/man1/htop.1.gz`) and the strings in `/usr/bin/htop`;
- the proc(5) pages and a live `/proc` on a 6.8 kernel (CLK_TCK 100, 4 KiB pages, 32 CPUs);
- Far's `@TaskList` help in `docs/upstream/far-manager/FarEng.hlf.m4` (main checkout);
- the rustix 1.1.5 and windows-sys 0.61.2 sources in the local cargo registry.

A fact marked *(knowledge)* could not be checked on this machine.

---

## 1. Goal and scope

### 1.1 What the user asked for

"Implement a processes viewer like htop in Linux." The standing quality bar has
three parts:

- The code must be as reliable as possible.
- Domain values are enums and validated newtypes, never bool or string flags
  and never raw integers.
- KISS and DRY apply, as set out in the global CLAUDE.md.

### 1.2 What exists today (verified)

- **Ctrl+W does not reach the `Overlay::Tasks` stub.** The route is:
  1. `src/action/menu.rs:15` maps Ctrl+W to `MenuCommand::Tasks`.
  2. `src/ops/menu.rs:102` hands that to `workflows::execute`
     (`src/ops/menu/workflows.rs:19`).
  3. `fn tasks` (lines 106-127) runs `ps -eo pid,comm,%cpu,%mem`, or
     `tasklist` on Windows.
  4. It opens `Viewer::report("Task list", …)` with the output.

  Three other routes reach the same function:
  - the F9 row `Task &list` (`src/menu/tables.rs:151`);
  - the Ctrl+Shift+w fallback (`src/action.rs:220`, then `dispatch/actions.rs:128`);
  - the console view (`dispatch/console.rs:35`).
- **The Change drive `Process list` row reaches the stub.** Main added it with
  NetBox: the Alt+F1/Alt+F2 dialog lists `DriveEntry::ProcessList`
  (`src/models/drive_entry.rs:58`) under the rule, and `select_drive`
  (`src/popup/drives.rs:76`) calls `ops::tasks::open`, which opens the stub.
  So the row leads nowhere useful until this viewer lands.
- **The stub.** `Overlay::Tasks(Box<TaskList>)` draws "Coming in this release".
  Its four files are `models/tasks.rs`, `ops/tasks.rs`, `popup/tasks.rs` and
  `ui/tasks.rs`. `ops::tasks::open` has two callers: the drive row and
  `src/popup/tasks/tests.rs`.
- **The report's `%CPU` misleads.** It is ps's average over the process's whole
  lifetime, not a live value.

### 1.3 First release (Linux, one squash-merged branch)

1. **Opening it.** Ctrl+W, Ctrl+Shift+w, the console view,
   F9 > Commands > Task list and the `Process list` row of the Change drive
   dialog (Alt+F1/Alt+F2) all open a full-screen live viewer on the existing
   `Overlay::Tasks` slot. It is one viewer: the drive row opens it, it is not
   a second entry.
2. **The table.**
   - Columns: `PID User PRI NI VIRT RES SHR S CPU% MEM% TIME+ Command`.
   - It refreshes every 1.5 s. The first CPU% appears 0.5 s after opening.
   - The cursor follows its process, not its row, across every refresh and
     every re-sort, once the user has moved it. Until then it keeps the first
     row: the first snapshot has no CPU% and is ordered by PID, and following
     its first row took the cursor to PID 1 in the middle of the list.
3. **Header meters.**
   - One CPU total bar.
   - Per-core bars. Cores are grouped only when their bars cannot fit.
   - Mem and Swp bars.
   - `Tasks`, `Load average` and `Uptime` lines.
4. **Sorting.**
   - F6, Ctrl+F12, `<` or `>` open the Sort by menu.
   - `I` inverts the order.
   - `P`, `M`, `T` and `N` sort by CPU%, memory, CPU time and PID.
   - The Far keys Ctrl+F3, F6, F7, F8, F10 and F11 sort directly.
   - A click on a column title sorts by that column.
5. **Marks.**
   - Space or Insert marks the process and moves down.
   - `c` marks the process and all its descendants.
   - `U` unmarks everything.
   - `*` inverts the marks.
   - Gray `+` and `-` select or deselect by a Far mask over process names,
     using the existing Select/Deselect dialog.
6. **Ending processes.**
   - F8 or Del opens a red confirmation: Terminate (SIGTERM) or Kill (SIGKILL).
   - F9 or `k` opens a Send signal menu, then the same confirmation.
   - Targets are frozen by process identity when the dialog opens.
   - Delivery re-checks each process's identity through a pidfd.
   - FFM itself, PID 1 and kernel threads are refused.
7. **Priority.** F7 opens a Priority form with a nice value. `]` and `[` step
   nice down or up by one.
8. **Finding processes.**
   - F4 or `\` opens a live filter bar: case-insensitive fixed strings, with
     `|` meaning OR.
   - `/` or Alt+letter opens a live search bar; F3 moves to the next match.
   - Typing digits jumps to that PID.
   - `u` opens a Show processes of menu.
   - `K` shows or hides kernel threads; they are hidden by default.
9. **Tree view.** F5, `t` or Ctrl+T switches between the list and the parent
   tree.
10. **Other keys.**

    | Key | Action |
    |---|---|
    | F3 or Enter | Process information |
    | F2 | Switch the last column between the command line and the name |
    | F11 or `Z` | Hold or resume the refresh |
    | Ctrl+R or Ctrl+L | Refresh now |
    | F1, `h` or `?` | Scrollable key help |
    | Left and Right | Scroll the Command column |
11. **Mouse.**
    - The wheel scrolls.
    - A click on a row selects it; a click on a column title sorts by it.
    - A click on the key bar presses that key, and a click on a menu line
      chooses it.
    - Dialog buttons work as in every other ffm dialog.
12. **Closing.** Esc, F10, `q`, Ctrl+C and Ctrl+W close the viewer. None of
    them ever quits ffm.
13. **Replay safety.** A replayed recording never sends a signal and never
    changes a priority.

### 1.4 Other platforms in the first release

Windows, macOS and the BSDs keep today's `ps` or `tasklist` report, from every
entry point in 1.3, the drive row included. It moves to `ops/tasks/report.rs`
and behaves exactly as before, until each platform has a native reader (phases
W and M in section 14). No supported platform loses anything at any merge.

### 1.5 Deferred (not in the first release)

- **Other platforms.**
  - A native Windows reader (Toolhelp and OpenProcess, a Terminate-only kill,
    priority classes) — phase W.
  - A macOS and BSD reader built on `ps` — phase M.
- **Settings and screens.**
  - Saving sort, refresh interval and toggles in `settings.yaml` (open question 1).
  - A setup screen (htop's F2) and configurable columns or meters.
- **Tree and threads.**
  - Collapsing tree nodes (htop's `+ - *` in tree view).
  - Userland thread rows (htop's `H`).
- **htop keys not carried over.**
  - Follow (`F`): the cursor already follows the process.
  - Full path (`p`), merged names (`m`).
  - strace (`s`), lsof (`l`), file locks (`x`), affinity (`a`),
    IO priority (`i`), autogroup nice (`{ }`).
  - Screen tabs (Tab), hiding containers (`O`).
  - Wrapped command-line and environment panes (`w`, `e`).
- **Columns and highlighting.**
  - IO-rate, cgroup and delay-accounting columns.
  - Highlighting new and dead processes.
- **Keys and entry points.**
  - Previous search match (Shift+F3 never reaches ffm on terminals without
    kitty keyboard support).
  - Ctrl+W from inside the F3 viewer or the F4 editor. Far allows it; with one
    Overlay it would destroy the open file.

### 1.6 Non-goals

- Signalling another machine.
- Showing `/proc/<pid>/environ` or `/proc/<pid>/io`. They are never read.
- Replacing `ops::menu::process::capture`. `archive` and `panel_details` still
  use it.

---

## 2. Screen

### 2.1 Layout

The sketch is 80x24, with 8 CPUs, kernel threads hidden, the default sort by
CPU% largest first, and two processes marked.

```
╔═════════════════════════════════ Task list ══════════════════════════════════╗
║CPU[|||||||         34.2%] Mem[||||||  38.2 G/125 G] Swp[             0/2.0 G]║
║ 0[||        12.0%]  1[|          8.1%]  2[||||||    40.3%]  3[          3.0%]║
║ 4[|          9.5%]  5[           2.2%]  6[|||       17.9%]  7[          1.4%]║
║Tasks: 132, 1204 thr, 180 kthr; 3 running  Load average: 0.52 0.58 0.59       ║
╟─────┬────────┬───┬───┬──────┬──────┬─┬─────┬─────┬─────────┬─────────────────╢
║ PID │  User  │PRI│NI │ VIRT │ RES  │S│CPU%▼│MEM% │  TIME+  │     Command     ║
║ 4242│bob     │ 20│  0│ 3.1 G│ 612 M│R│ 38.4│  1.9│ 12:04.55│/usr/lib/firefox}║
║  812│bob     │ 20│  0│ 1.2 G│ 301 M│S│  6.0│  0.9│  3:11.02│/usr/bin/gnome-s}║
║    1│root    │ 20│  0│22.4 M│14.5 M│S│  0.0│  0.0│  1:32.07│/sbin/init splash║
║ 2001│bob     │ 25│  5│ 7.4 M│ 4.0 M│S│  0.0│  0.0│  0:00.00│sleep 30         ║
║     │        │   │   │      │      │ │     │     │         │                 ║
║     │        │   │   │      │      │ │     │     │         │                 ║
║     │        │   │   │      │      │ │     │     │         │                 ║
║     │        │   │   │      │      │ │     │     │         │                 ║
║     │        │   │   │      │      │ │     │     │         │                 ║
║     │        │   │   │      │      │ │     │     │         │                 ║
║     │        │   │   │      │      │ │     │     │         │                 ║
║     │        │   │   │      │      │ │     │     │         │                 ║
║     │        │   │   │      │      │ │     │     │         │                 ║
╟─────┴────────┴───┴───┴──────┴──────┴─┴─────┴─────┴─────────┴─────────────────╢
║4242 firefox: /usr/lib/firefox/firefox -contentproc -childID 12 -isForBrowse} ║
╚═══════════════════════ 132 of 312 processes, 2 marked ═══════════════════════╝
1Help  2Name  3Info  4Filter5Tree  6Sort  7Nice  8Kill  9Sign…10Quit11Hold12
```

The view is full screen, like the F3 viewer and the F4 editor: `Clear` over
`f.area()`, then a `far_block` double frame. It is not a centred dialog. It
therefore moves from `render_commands_dialog` (`ui/overlays.rs:314`) to the
top-level `render_overlays` match, beside `Viewer` and `Editor`.

`apply_theme` runs before overlays (`ui.rs` draw order). The Colors and
Highlighting settings therefore do not recolour the viewer, just as they do not
recolour the F3 viewer or the F4 editor.

### 2.2 Rows, top to bottom

| Row | Content |
|---|---|
| Frame top | `far_block` title. See 2.7 for the variants. |
| Summary row 1 | CPU total, Mem and Swp bars in thirds, with one blank between them. |
| Per-core rows | Four bars per row. Bar widths share the interior as `fkey_widths` shares the key bar (remainder to the left). |
| Summary row 2 | `Tasks: …`, `Load average: …`, `Uptime: …`, separated by two blanks. Fields that do not fit are dropped from the right (at 80 columns Uptime goes). |
| Rule | `╟─┬─╢`. The column separators join the frame, as in `panel.rs` `column_rule`. |
| Column titles | Titles centred with `center_cells`. The sort column's title is in cursor colours, followed by `▲` or `▼` when it fits. |
| Table body | Only the visible window, `crate::scroll::visible_window(len, page, offset, cursor)`, is built. |
| Closing rule | `╟─┴─╢`. |
| Info row | The cursor process as `pid name: command line`, clipped with Far's `}`. A notice replaces it until the next key (2.7). |
| Frame bottom | The centred footer (2.7). |
| Last row | The key bar, or a text bar while filtering, searching or typing a PID (2.7). |

### 2.3 Header shape

The header shape is one function shared by the renderer, paging and the mouse.
It lives in `models/tasks/header.rs`, because `popup` pages by it and `models`
may not depend on `ui`. The model knows only the terminal's height (`Viewport`),
so the shape depends on the row count and the number of CPUs, never on the width.

```
FRAME_CHROME   = 6   // top border, titles, closing rule, info row, bottom border, key bar
METER_RULE     = 1   // the ╟┬╢ rule, present only when meters are shown
SUMMARY_ROWS   = 2
MIN_TABLE_ROWS = 5
BARS_PER_ROW   = 4

summary  = SUMMARY_ROWS if rows >= FRAME_CHROME + METER_RULE + SUMMARY_ROWS + MIN_TABLE_ROWS (14) else 0
budget   = 0 if summary == 0 or cores <= 1
           else min(rows / 3 - SUMMARY_ROWS, rows - 14)            // saturating
per_bar  = ceil(cores / (BARS_PER_ROW * budget))                    // cores averaged into one bar
bars     = ceil(cores / per_bar)
cpu_rows = ceil(bars / BARS_PER_ROW)
meters   = summary + cpu_rows
chrome   = FRAME_CHROME + meters + (METER_RULE if meters > 0 else 0)
page     = max(1, rows - chrome)
```

`cores` is the number of per-core counters in the latest snapshot. It is 0 while
the first sample is pending, so the header can grow once when that sample
arrives. A grouped bar is labelled with its core range (`0-1`). Its value treats
the group as one CPU: the sum of the members' deltas over the sum of their
totals, computed by `CpuMeters::grouped(per_bar)` in models.

| Rows × CPUs | per_bar | cpu_rows | chrome | page |
|---|---|---|---|---|
| 24 × 8 | 1 | 2 | 11 | 13 |
| 24 × 32 | 2 | 4 | 13 | 11 |
| 50 × 32 | 1 | 8 | 17 | 33 |
| 24 × 1 | – | 0 | 9 | 15 |
| 18 × 8 | 1 | 2 | 11 | 7 |
| 14 × 8 | – | 0 | 9 | 5 |
| 13 × 8 | – | 0 (no meters) | 6 | 7 |
| 4 × 8 | – | 0 | 6 | 1 (the renderer clips) |

### 2.4 Columns (`ui/tasks/table.rs`)

| Column | Title | Width | Alignment | Sort key | Dropped |
|---|---|---|---|---|---|
| PID | `PID` | max(5, digits of the largest PID in the snapshot) | right | Pid | never (the user checks it before a kill) |
| User | `User` | 8 | left, Far `}` clip (`clip_name`) | User | 7th |
| PRI | `PRI` | 3 | right, `RT` at −100 | Priority | 2nd |
| NI | `NI` | 3 | right | Priority | 6th |
| VIRT | `VIRT` | 6 | right, `abbreviate_bytes` | Virtual | 3rd |
| RES | `RES` | 6 | right | Resident | 9th |
| SHR | `SHR` | 6 | right | Shared | 1st |
| S | `S` | 1 | state letter | State | 8th |
| CPU% | `CPU%` | 5 | right | Cpu | never |
| MEM% | `MEM%` | 5 | right | Resident (the same order) | 4th |
| TIME+ | `TIME+` | 9 | right | CpuTime | 5th |
| Command | `Command`, or `Name` after F2 | the rest, at least 16 | left, `}` clip, Left/Right scroll | Command or Name | never |

- Columns are dropped in the order above until Command keeps 16 cells.
- At 80 columns with a 5-digit PID column only SHR is dropped. Command then gets
  17 cells, as in the sketch. With 7-digit PIDs, PRI is dropped too. At 120
  columns every column is shown.
- A value the platform did not give shows as `-`.
- Every name, user and command line goes through `ui::printable_name` before it
  is measured. Every width is `Cells`; `str::len` is never used.
- In tree view, the Command cell starts with `├─ `, `└─ ` and `│  ` guides built
  from `TreeLine`.
- Left and Right scroll only the Command column, by `ScrollLeft` cells, as Far's
  viewer scrolls (Ctrl+Left and Ctrl+Right by 20).

### 2.5 Formats (pure functions in `ui/tasks/format.rs`, tested to the character)

| Value | Format |
|---|---|
| CPU%, MEM% | Below 100: one decimal (`38.4`, `0.0`). From 100: a whole number (`3200`). `CpuUsage::Pending`: `-`. |
| TIME+ | Under 1 h: `M:SS.hh` (`1:32.07`, `12:04.55`). Under 100 h: `HhMM:SS` (`12h04:55`). From 100 h: `DdHHh` (`4d05h`). Saturating. |
| Uptime | `Uptime: 05:52:06`, `Uptime: 1 day, 05:52:06`, `Uptime: 12 days, 05:52:06` |
| Load | `Load average: 0.52 0.58 0.59` |
| Tasks | `Tasks: 132, 1204 thr, 180 kthr; 3 running`. The `; N running` part is omitted when unknown. |
| Bytes | `abbreviate_bytes`, Far style: `381 K`, `22.4 M`, `3.1 G`. Deliberately not htop's `381M`. |
| Meter text | CPU `34.2%`; Mem `38.2 G/125 G`; Swp `0/2.0 G`. |
| PRI | The kernel priority. `RT` when it is −100 (SCHED_FIFO or SCHED_RR at rt_priority 99). |
| S | The kernel's letter (`R S D Z T t X I P`). Any other byte shows as itself when it is printable ASCII, otherwise `?`. |

### 2.6 Colours (`ui/palette.rs`; every colour is a named const)

- The body is `FAR_BLUE`, with data in `DATA` and column rules and meter
  brackets in `FRAME`.
- The Command column is `FILE`. Kernel threads, when `K` shows them, are
  `HIDDEN`.
- Marked rows are `MARKED` throughout.
- The cursor bar is the panel's `cursor_style(Mark)`: `CURSOR_BG`, bold,
  and yellow text on a marked row.
- The sort column title is `CURSOR_BG`/`CURSOR_FG`.

The palette gains these constants. Each is named by concept, and each is an
alias of an existing colour where one fits.

| Constant | Meaning |
|---|---|
| `METER_USER` | green: CPU user, memory used |
| `METER_NICE` | blue: CPU nice, memory buffers |
| `METER_KERNEL` | red: CPU system+irq+softirq, swap used |
| `METER_VIRT` | cyan: steal and guest |
| `METER_SHARED` | magenta: shared memory |
| `METER_CACHE` | yellow: page cache, swap cache |
| `STATE_ALERT` | red: the `D` and `Z` state letters |

### 2.7 Title, footer, info row and bottom-row variants

- **Title.** ` Task list `. When one of these conditions applies, a marker is
  added, highest precedence first:

  | Condition | Marker |
  |---|---|
  | Stalled | ` · not responding ` |
  | Refresh failed | ` · refresh failed ` |
  | Overdue | ` · stale ` |
  | Held | ` · held ` |
- **Footer.**
  - ` 312 processes ` when every process is shown.
  - Otherwise ` 132 of 312 processes `, plus `, 2 marked`, `, 3 unreadable`,
    `, filter fire|chr` and `, user bob` when they apply.
  - The footer is built by one pure function.
- **Info row.** It shows the cursor process, or a `Notice` until the next key.
  A notice uses `DIALOG` colours for information and `WARNING` colours for a
  refusal or failure.
- **Bottom row.**

  | State | Content |
  |---|---|
  | Browse | The key bar, `render_key_bar(f, area, &labels)` (section 4). Labels: `1Help 2Name 3Info 4Filter 5Tree 6Sort 7Nice 8Kill 9Signal 10Quit 11Hold 12`. `2` reads `Cmd` while names are shown, `5` reads `List` in tree view, and `11` reads `Live` while held. |
  | Filter bar | ` Enter Done  Esc Clear  Filter: fire|chr`, with the caret placed. |
  | Search bar | ` F3 Next  Enter Done  Esc Cancel  Search: pyth`. The query is drawn in `PROMPT_FAILED` when no row matches. |
  | PID jump | ` Enter Done  Esc Cancel  PID: 42` |

  At 80 columns the 9th cell is 6 wide, so `Signal` shows as `Sign…`. That is
  the same clipping Far's own bar has at 80 columns.

### 2.8 Nested dialogs

Every nested dialog is drawn over the live view, and the refresh continues
behind it.

| Dialog | Opened by | Drawn with | Content |
|---|---|---|---|
| Terminate | F8, Del | `dialog::render_warning` (red) | `Do you wish to terminate the process?` or `… the 3 processes?`, then the targets through `dialog::preview_lines` (first five, then `… and N more`). It adds `Skipped: FFM itself` when targets were refused, and `FFM runs inside 812 bash; ending it ends this session.` for an ancestor, when the signal ends a process (`…; pausing it freezes this session.` for SIGSTOP and SIGTSTP; nothing for SIGCONT and SIGWINCH, which end nothing). Buttons `{Terminate} [Kill] [Cancel]`, with Terminate focused. |
| Send signal | F9, `k` | `ui/menu_dialog/choices.rs` (teal menu) | Title `Send signal to 4242 firefox` or `… to 3 processes`. One row per offered signal (`SIGTERM   ask to terminate`), with SIGTERM under the cursor. Enter opens a red confirmation: `Send SIGHUP to …?` `{Send} [Cancel]`. |
| Sort by | F6, Ctrl+F12, `<`, `>` | choices menu | One row per sort key with its shortcut (`√ CPU%   P`), a tick on the key in force, then `Reverse order   I`. |
| Show processes of | `u` | choices menu | `All users`, then each owner in the snapshot with a count (`bob (57)`). The scope in force is ticked. |
| Priority | F7 | `dialog::render_tracked`, `Ground::Prompt` | A Text row with the target(s), Field `Nice:` with the current value, Label `-20 runs first, 19 runs last`. Buttons `{Set} [Cancel]`; Set is disabled while the value is not a `Niceness`. |
| Select / Deselect | Gray `+`, `-` | the existing `render_mask_prompt` | The Far mask dialog, offering the cursor process's name as the mask. |
| Process info | F3, Enter | `render_tracked`, labels only | PID, Parent, Name, Command (wrapped, at most 4 rows), User, State, Priority/Nice, Threads, Running for, CPU time, VIRT/RES/SHR. Button `{OK}`. Built from the snapshot, with no extra I/O. |
| Error | after a failed delivery | `render_warning` | When some targets took it, first the sentence saying so (`Sent SIGTERM to 1 process.`); then one line per failed target, through `preview_lines`. Button `[OK]`. |
| Help | F1, `h`, `?` | the body area, in `HELP_FG`/`HELP_BG` | The rows of `action::tasks::HELP`, scrollable (`ScrollTop`). |

Tracking rules:

- When no tracked dialog is drawn, `ui::tasks::render` calls `tracker.clear()`.
- Whenever the nested modal changes kind, `popup::tasks` calls
  `app.dialog_mouse.reset()`, so each dialog opens centred.

### 2.9 Small terminals

- Every rectangle is computed with saturating arithmetic.
- `tests/degenerate_size_test.rs` draws the view at every size from 0x0 to 24x24,
  both empty and with a populated snapshot.
- Below 14 rows the meters are dropped (2.3). Below 10 columns only the key bar
  is drawn.

---

## 3. Keymap

**How clashes are settled.** Inside the viewer, a key keeps the meaning Far or
ffm gives it wherever that meaning makes sense for a process list. Keys that
Far leaves empty take htop's meaning. When htop's meaning loses, it lands on a
harmless dialog: no reflex from either program can end a process without a
second, explicit step.

`action::tasks::normalize` runs first. It turns `Char(lowercase)` + SHIFT into
`Char(uppercase)` with no SHIFT. This matters because terminals and the BDD
harness send an uppercase letter either way (section 10.9).

### 3.1 Browse mode (`action::tasks::resolve` → `TaskCommand`)

| Keys | Action | htop | Far / ffm |
|---|---|---|---|
| Up, Down, PgUp, PgDn, Home, End, mouse wheel | Move the cursor. A page is `TaskList::page_for(app.viewport)`. The cursor is a process identity, so it stays on its process across refreshes. | same keys; Alt-k/j too | same |
| Left, Right / Ctrl+Left, Ctrl+Right | Scroll the Command column by 1 or by 20 cells | scroll the list | the F3 viewer's keys |
| Space, Insert | Toggle the mark on the cursor process and move down | Space tags | Insert marks |
| `c` | Mark the cursor process and all its descendants in the snapshot | `c` | — |
| `U` | Unmark all | `U` | — |
| `*` (without Alt) | Invert the marks of the shown rows | expand/collapse all (tree) | Gray `*` inverts |
| `+`, `-` (without Alt) | Select / Deselect dialog: a Far mask (`crate::mask::Mask`) over process names | expand/collapse subtree | Gray `+`/`-` select or deselect by mask |
| Enter, F3 | Process info dialog | F3 search | F3 View |
| F1, `h`, `?` | Task list help page | same | F1 Help |
| F2 | Switch the last column between command line and name | F2 Setup | Far Task list F2 switches caption and path |
| F4, `\` | Filter bar (3.2) | same | — |
| `/`, Alt+character | Search bar (3.2); Alt+character opens it with that character typed | `/`; Alt-h/j/k/l move | Alt+letter quick search |
| `0`-`9` | PID jump (3.2) | "Numbers: PID search" | — |
| F5, `t`, Ctrl+T | Switch list and tree | F5, `t` | Ctrl+T tree |
| F6, Ctrl+F12, `<`, `>` | Sort by menu | F6, `<`, `>` open the sort selector (man page) | Ctrl+F12 |
| Ctrl+F3 / F6 / F7 / F8 / F10 / F11 | Sort by Name / Memory / PID / Start time / Command / User. The key already in force reverses the order. | — | panel direct sort by analogy: name, size, unsorted, creation, description, owner |
| `P`, `M`, `T`, `N` | Sort by CPU% / memory / CPU time / PID. The key already in force reverses. | same | — |
| `I` | Invert the order | same | — |
| `u` | Show processes of menu | same | — |
| `K` | Show / hide kernel threads | same | — |
| F7 | Priority dialog for the marked processes or the cursor process | Nice − at once | MkFold (no process meaning) |
| `]`, `[` | Nice −1 (raise priority) / nice +1, marked or cursor. `]` acts at once; `[` acts at once for root and asks first for anyone else, who cannot undo it (resolved question 6) | same | — |
| F8, Del | Terminate confirmation | F8 Nice + | F8 Delete; Far Task list Del kills |
| F9, `k` | Send signal menu | same | F9 menu bar (not meaningful inside a modal) |
| F11, `Z` | Hold / resume the refresh | `Z` | F11 plugins (no process meaning) |
| Ctrl+R, Ctrl+L | Refresh now (while held: one sample, then held again) | Ctrl-L | Far Task list Ctrl+R |
| Esc, F10, `q`, Ctrl+C, Ctrl+W | Close the viewer. Never quits ffm. | F10, `q` quit htop | Esc/F10 close; Ctrl+W toggles |
| any other key | Consumed (`TaskCommand::Ignore`); nothing falls through to the panels | — | — |

Ctrl+F4, Ctrl+F5 and Ctrl+F9 are unbound: extension, modification time and
access time have no process analogue. Ctrl+F3 and Ctrl+F12 never reach ffm on a
terminal without the kitty keyboard protocol
(`hotkeys-and-parity.md:146`), so F6, `<`, `>` and a title click are the
reliable routes to sorting.

### 3.2 Text bars (filter, search, PID)

In all three bars the editing keys are the shared
`popup::menu_dialog::edit(line, code, modifiers)`: characters, Backspace,
Delete, Left, Right, Home and End. A bracketed paste inserts its text with
control characters removed, capped at 256 characters (section 4.4 has the
routing).

- **Filter.**
  - Every edit re-parses `FilterTerms` and rebuilds the view.
  - Up, Down, PgUp and PgDn still move the cursor.
  - Enter keeps the filter and closes the bar. Esc clears the filter and closes
    the bar.
- **Search.**
  - Every edit moves the cursor to the first row, from the cursor down and
    wrapping, whose shown text contains the query, case-insensitively.
  - F3 moves to the next match.
  - Enter or Esc closes the bar and leaves the cursor where it is.
  - Any other navigation key closes the bar and then acts as in Browse.
- **PID jump.**
  - A digit appends (at most 10) and moves the cursor to that exact PID when it
    is shown.
  - Backspace removes a digit. Enter or Esc ends the jump.
  - Any other key ends the jump and is then handled as in Browse.

### 3.3 Nested dialogs and menus

| Where | Keys |
|---|---|
| Menus (Sort by, Send signal, Show processes of) | Up, Down, PgUp, PgDn, Home, End through the shared `popup::menu_dialog::navigate`; Enter chooses; Esc and F10 cancel. In Sort by, `I` chooses Reverse order. |
| Terminate / Send confirmation | Enter presses the focused button; Tab, Left and Right move focus; `t` Terminate (`s` Send), `k` Kill, with or without Alt but never with Ctrl; Esc, F10 and `c` Cancel. |
| `[` confirmation | Enter presses the focused button (Renice first); Tab, Shift+Tab, Left and Right move focus; Esc and F10 cancel. |
| Priority form | Field editing through `menu_dialog::edit`; Up and Down step the value within −20..=19; Tab moves between field and buttons; Enter presses Set (only when valid); Esc cancels. |
| Select / Deselect | Exactly the panel dialog's keys, through the shared `popup::mask_prompt::mask_key` (4.4). |
| Info, Error | Enter, Esc, F10 and F3 close. |
| Help page | Up, Down, PgUp, PgDn, Home, End scroll; Esc, F1, F10, `q`, `h` and `?` close. |

### 3.4 Mouse

`ui::task_click_at(screen, &TaskList, column, row) -> Option<TaskClick>` answers
from the same `layout()` function the renderer draws with. The
`click_special_dialog` branch for `Overlay::Tasks` turns each click into the key
or transition that already does the job.

| Clicked | Effect |
|---|---|
| Table row (Browse) | `TaskList::select_index(i, page)` |
| Column title (Browse) | `TaskList::sort_clicked(SortKey)`: that key, or the reverse when it is already in force |
| Key-bar cell (Browse) | Dispatches `KeyCode::F(n)`, with `n` from `mouse::fkey_at(width, 12, column)` |
| Menu line | Moves the menu cursor, then dispatches Enter (the `click_sort_menu` precedent) |
| Buttons of a tracked dialog | The generic `DialogMouse::hit` → `dialog_button_key` path, with a new `Overlay::Tasks` arm (4.4) |
| Wheel | Up/Down keys, through the existing mapping in `dispatch_dialog_mouse` |

A click never signals anything. The most a click can do is press F8 or F9, and
both only open a dialog.

### 3.5 Clashes and how each is settled

| Key | htop | Far / ffm | Decision |
|---|---|---|---|
| F3 | incremental search | View | Info (Far); search moves to `/` and Alt+letter |
| F8 | nice + | Delete | Terminate confirmation (Far); nice + stays on `[` |
| F7 | nice − | MkFold | Priority dialog (harmless); nice − stays on `]` |
| F2 | Setup | user menu; Far Task list: caption/path | Name / command line (Far Task list) |
| F9 | Kill menu | menu bar | Send signal menu (htop) |
| F10, `q` | quit htop | F10 quits ffm | close the viewer only |
| Esc | nothing at top level | closes the overlay | closes the viewer, after ending an open bar |
| Ctrl+C | ends htop | quits ffm | closes the viewer only |
| Del | — | Far Task list: kills at once | opens the Terminate confirmation |
| `+ - *` | tree expand/collapse | Gray select/deselect by mask, invert | Far meaning; tree collapse is deferred |
| Alt+letter | Alt-h/j/k/l move | quick search | search bar prefilled (Far) |
| `<` `>` | open the Sort by selector (man page) | — | open the Sort by menu, as htop does |
| Ctrl+L | refresh | Information panel | refresh now (the panel meaning does not apply here) |
| Shift+F3 | previous match | never arrives on non-kitty terminals | not bound |
| `F` (follow) | cursor follows the process | — | not bound: the cursor always follows the process |

---

## 4. Architecture

### 4.1 Data flow

```
bootstrap ── installs ─▶ App.task_host : TaskHostSlot(Option<Arc<dyn ProcessHost>>)
                          (replay wraps the host so it cannot signal: recording/processes.rs)

Ctrl+W / F9 row ─▶ MenuCommand::Tasks ─▶ ops::menu ─▶ ops::tasks::open
Change drive Process list row ─▶ popup::drives::select_drive ─▶ ops::tasks::open
    host installed ─▶ Overlay::Tasks(TaskList::new(session, own pid))
    no host        ─▶ ops::tasks::report (today's ps/tasklist Viewer report)

LoopState::pump_background ─▶ application::task_jobs::TaskJobs::pump(app)
    ├─ worker thread "ffm-tasks": ProcessReader::sample() ─▶ Result<Sample, SampleError>
    └─ TaskList::apply_sample(result, now)   (models: CPU%, meters, view, cursor)

key ─▶ popup::tasks ─▶ action::tasks::resolve ─▶ TaskList transitions
                                              └▶ ops::tasks::control ─▶ ProcessHost::signal / set_priority
                                                                         (UI thread, after confirmation)
draw ─▶ ui::tasks::render(f, tracker, &TaskList)       reads the model only
click ─▶ dispatch/dialog_mouse ─▶ ui::task_click_at ─▶ key or transition
```

- Sampling runs on a worker thread, because a `/proc/<pid>/cmdline` read can
  block behind the target's mmap lock *(knowledge; kernel behaviour)*.
- Kill and renice run on the UI thread in `ops`. They read only
  `/proc/<pid>/stat` and make short syscalls, so they still work while a sample
  read hangs. That is the moment a user most needs to end the hung process.

### 4.2 Layering (enforced)

- `ui` reads `models`, ui helpers, `crate::mouse`, `crate::scroll` and
  `crate::action::tasks::HELP`. It never reads `/proc` and never calls a platform
  API.
- `models` defines the ports (`ProcessHost`, `ProcessReader`) and derives every
  value, but performs no I/O. The own PID is passed in by `ops`, because
  `models` may not use `std::process`.
- `action::tasks` is pure: no `std::io` and no `std::env`.
- `popup` calls `models` and `ops`.
- `ops` calls `models` and the installed host.
- `process_table` (the adapter) calls `platform`, `directory::owner` and `std::fs`.
- `platform` is the only place that calls rustix process APIs. It takes the
  validated `models` types (`Pid`, `Signal`, `Niceness`), so no other module can
  build a raw PID.
- `application` owns the thread. `App` and `Overlay` stay free of threads,
  channels and receivers; both derive `Clone`.
- `tests/module_boundaries/layers.rs` gains:
  - `"crate::process_table"` and `"crate::platform"` in `RENDERING_FORBIDDEN`
    (15 to 17). This applies to `ui` and, through the concatenation, to `action`.
  - `"crate::process_table"` in `MODELS_FORBIDDEN` (13 to 14). `models` keeps
    its one existing `crate::platform` use in `drive_entry.rs`.
- No new unsafe on Linux or macOS. Phase W adds exactly one reviewed
  `BOUNDARIES` file.

### 4.3 Files to add

Every file starts with a `//! Purpose:` line, stays at 300 lines or fewer, and
has its tests in sibling `tests*.rs` files.

| File | Responsibility |
|---|---|
| `src/models/tasks.rs` (rewritten) | `TaskList`: private fields, `Clone + Default`, **no `Debug`** (it holds command lines, which may contain secrets). Accessors for ui; the entry point of every transition; `page_for(Viewport)`, `header_shape(Viewport)`, `clamp_to(Viewport)`. |
| `src/models/tasks/identity.rs` | `Pid`, `StartTime`, `Start`, `ProcessKey`, `TaskSession` |
| `src/models/tasks/units.rs` | `Percent`, `MemoryBytes`, `Load`, `LoadAverage`, `CpuUsage` |
| `src/models/tasks/priority.rs` | `Niceness`, `NiceStep`, `KernelPriority`, `PriorityClass`, `SettableClass`, `Priority`, `PriorityChange` |
| `src/models/tasks/signal.rs` | `Signal`, `Signal::OFFERED` (per cfg), `KillOffer` |
| `src/models/tasks/sample.rs` | Raw sample types, the `ProcessHost` and `ProcessReader` ports, `SampleError`, `SystemPart`, `ControlError` |
| `src/models/tasks/snapshot.rs` | `Baselines`, `Snapshot::derive(&Baselines, Sample) -> (Snapshot, Baselines)`, `ProcessRow` |
| `src/models/tasks/meters.rs` | `CpuBar`, `CpuMeter`, `CpuMeters::grouped`, `MemoryMeter`, `SwapMeter`, `TaskCounts` |
| `src/models/tasks/view.rs` | `SortKey`, `Sort`, `Arrangement`, `CommandView`, `KernelThreads`, `UserScope`, `FilterTerms`, `ViewRow`, `build_view` |
| `src/models/tasks/tree.rs` | Iterative parent tree: orphans, the start-time rule, a visited set |
| `src/models/tasks/selection.rs` | Cursor (`ProcessKey` + index), offset, marks (`BTreeSet<ProcessKey>`), moves, search, PID jump, mask marking |
| `src/models/tasks/schedule.rs` | `Cadence`, `Schedule`, `Request`, `Health`, and the constants `REFRESH_INTERVAL`, `WARM_UP`, `OVERDUE_AFTER`, `STALL_LIMIT` |
| `src/models/tasks/header.rs` | `HeaderShape::for_screen(cores, Viewport)`, chrome and page rows (2.3) |
| `src/models/tasks/modal.rs` | `Modal`, `TextBar`, `SearchBar`, `Found`, `PidQuery`, `TaskMenu`, `MenuCursor`, `KillConfirm`, `PriorityForm`, `Notice` |
| `src/models/tasks/targets.rs` | `Target`, `Relation`, `Refusal`, `Skipped`, `freeze_targets`, `refusal` |
| `src/models/tasks/host.rs` | `TaskHostSlot` (host plus session counter) |
| `src/models/tasks/tests_support.rs` | `cfg(test)` builders for samples and snapshots, shared by the models, ui, popup and ops tests |
| `src/process_table.rs` | Private `mod process_table;` in lib.rs. `SystemHost` (Linux). Same-signature `#[cfg]` `system_host() -> Option<Arc<dyn ProcessHost>>`: `Some` on Linux, `None` elsewhere until phases W and M. |
| `src/process_table/procfs.rs` | `ProcFs { root, ticks: ClockTicks, page: PageSize, users: Owners, commands: CommandCache }` implements `ProcessReader`. Compiled under `cfg(any(target_os = "linux", test))`, so the Windows CI runs its tests. `ProcFs::system()` is `cfg(target_os = "linux")`. |
| `src/process_table/procfs/parse.rs` | Pure parsers for stat, statm, status, cmdline, /proc/stat, meminfo, loadavg and uptime |
| `src/process_table/procfs/control.rs` | Identity-checked signal and renice. The decisions are pure (`classify_open`, `verify_start`, `control_error`) under `cfg(any(target_os = "linux", test))`; the orchestration is `cfg(target_os = "linux")`. |
| `src/action/tasks.rs` | `TaskCommand`, `ListMotion`, `normalize`, `resolve` (built with `keymap!`, final `_ => Ignore`), and `HELP: &[(&str, &str)]`, the help page rows |
| `src/popup/tasks/browse.rs` | Interprets `TaskCommand` exhaustively |
| `src/popup/tasks/bars.rs` | Filter, search and PID bars |
| `src/popup/tasks/dialogs.rs` | Confirmation, priority, info, error, help and mask keys |
| `src/popup/tasks/menus.rs` | Sort by, Send signal and Show processes of |
| `src/ops/tasks/report.rs` | Today's report, moved. Same-signature `#[cfg(windows)]` / `#[cfg(not(windows))]` `report_command() -> Command`, replacing the `if cfg!()` in `workflows.rs:107-114`. |
| `src/ops/tasks/control.rs` | Freezes targets and opens the confirmation; delivers signals; nice steps and the priority form; outcomes become a `Notice` or `Modal::Error`; afterwards the schedule is set to `Due` |
| `src/application/task_jobs.rs` | `TaskJobs` controller and the `ffm-tasks` worker (section 7) |
| `src/application/dispatch/dialog_mouse/tasks.rs` | Click translation (3.4) and `tasks_button_key` |
| `src/recording/processes.rs` | `Input::process_host(inner) -> Arc<dyn ProcessHost>`: live input returns `inner`; replay wraps it in `ReadOnlyHost`, whose `signal` and `set_priority` return `ControlError::ReadOnly` |
| `src/ui/tasks.rs` (rewritten) | `render(f, tracker, &TaskList)`, `layout(screen, &TaskList) -> TaskLayout`, `pub(crate) fn task_click_at`, `TaskClick` |
| `src/ui/tasks/meters.rs` | Pure span builders for bars and summary lines |
| `src/ui/tasks/table.rs` | `Column`, `Layout::fit`, title line, rows of the visible window |
| `src/ui/tasks/format.rs` | Pure formatters (2.5) |
| `src/ui/tasks/chrome.rs` | Title, footer, info row, key labels, bottom bars |
| `src/ui/tasks/dialogs.rs` | Nested dialogs and the help page |
| `src/test_support/processes.rs` | `FixtureHost`, `FixtureReader`, `GatedReader` and sample builders, under `cfg(any(test, feature = "test-helpers"))` |
| `tests/tui_features/tasks.feature`, `tests/tui_bdd/tasks.rs` | BDD scenarios and their steps |
| `benches/tasks_bench.rs` | Criterion: `Snapshot::derive` and `build_view` over 2000 synthetic processes; `ProcFs::sample` over a 2000-entry fake procfs |

### 4.4 Files to change

| File | Change |
|---|---|
| `src/scroll.rs` | Gains `pub fn visible_window(len, rows, offset, selected) -> Range<usize>`, moved from `ui/panel.rs:860`. Its tests move with it. `ui/panel.rs:695`, `ui/panel/modes.rs:35` and `ui/tree.rs:58` import it from here. |
| `src/ui/fkey_bar.rs` | `render_command_bar(f, area)` becomes `render_key_bar(f, area, labels: &[(&str, &str); FKEY_COUNT])`: the array type admits only the twelve cells the hit test resolves. `ui.rs:217` passes `&FKEY_LABELS`. The widths stay `mouse::fkey_widths`. Update the tests at `ui/tests.rs:373` and `ui/tests_render_widths.rs:874`. |
| `src/ui/menu_dialog/choices.rs` | `render` and `row_at` become generic over `rows: &[impl AsRef<str>]`. `models::MenuChoice` implements `AsRef<str>` (its label). The module becomes reachable from `ui::tasks`. |
| `src/popup/menu_dialog.rs` | `navigate` becomes `pub(super)` for the tasks menus. |
| `src/popup/mask_prompt.rs` | Extract `pub(super) fn mask_key(&mut MaskPrompt, code, modifiers) -> MaskKey { Editing, Accepted(MaskPurpose, Mask), Cancelled }`. The panel handler and the tasks handler both use it. Resolves the file's own TODO. |
| `src/ui/overlays.rs` | The Tasks arm moves from `render_commands_dialog` (317) to `render_overlays`. `render_mask_prompt` becomes `pub(super)`. `HELP_LINES` gains `"Ctrl+W: Task list (processes; F1 inside lists its keys)"` and merges `:winmsg` and `:winperf` into one line, so the unscrolled box stays at 22 rows. |
| `src/ui/dialog.rs` | `pub(super) fn preview_lines(items: &[T], name: impl FnMut(&T) -> String, into: &mut Vec<String>)`: only the first five items are named, and the `… and N more` count comes from the same slice, so the two cannot disagree. `DELETE_PREVIEW` moves here from `overlays.rs:186`. Used by Delete, the Terminate confirmation and the Error dialog. |
| `src/ui/panel.rs` | `center_cells`, `cursor_style`, `cursor_colour`, `separator_style` and `COLUMN_SEPARATOR` become `pub(super)`. They are reused, not copied. `cursor_style` and `cursor_colour` take `Mark { Marked, Unmarked }`, not a `bool`. |
| `src/ui/palette.rs` | New constants (2.6) |
| `src/ui.rs` | `mod tasks;` stays. Adds `pub(crate) use tasks::{task_click_at, TaskClick};`. |
| `src/ui/tests_stub_dialogs.rs` | Remove `Overlay::Tasks` from `every_menu_dialog` (line 38): a full-screen view records no dialog area. |
| `src/models.rs` | Re-export the tasks types that `ui`, `popup`, `ops`, `application`, `platform`, `process_table` and `test_support` use. |
| `src/models/app.rs` | `pub task_host: TaskHostSlot`, initialised in `with_panels` (the one place defaults are listed). `Clone` keeps the host: it is immutable and shared, not an `Arc<Mutex>`. |
| `src/models/app_overlay.rs` | `set_viewport` also clamps an open `TaskList` (`list.clamp_to(viewport)`), as it does for `Viewer`. |
| `src/models/overlay.rs` | Doc comment of `Tasks` only |
| `src/directory/owner.rs` | `pub fn name_of_uid(&mut self, uid: u32) -> Option<String>`; `of()` delegates to it. There is no second `getpwuid_r`. The cache tells a failed lookup from a missing entry, and `pub fn forget_failed_lookups(&mut self)` drops the failures so the reader retries them once per sample (6.1.2). |
| `src/platform.rs`, `src/platform/unix.rs` | See 6.1.4 for the wrapper list. |
| `Cargo.toml` | Add rustix feature `"param"` to `[target.'cfg(unix)'.dependencies]` (it is `param = []`, so no crate is added and `Cargo.lock` does not change). Add a `[[bench]]` for `tasks_bench`. |
| `src/lib.rs` | `mod process_table;` |
| `src/action.rs` | `pub mod tasks;` beside `pub mod wsl;` |
| `src/action/commands.rs:132` | Doc of `ToolCommand::Tasks`: "the live process viewer" |
| `src/popup.rs` | `Overlay::Tasks(_) => tasks::process(…)` stays. The paste export becomes `pub(crate) fn paste_text(app, text)`, which routes to the editor (today's `paste_editor_text`) or to an open Task list bar. |
| `src/popup/tasks.rs` (rewritten) | A router by `Modal`. It clears the notice on every key, resets `dialog_mouse` when the modal kind changes, and always returns `true`. |
| `src/ops/tasks.rs` (rewritten) | `pub fn open(app) -> io::Result<()>`: with a host installed it opens the viewer with a new `TaskSession` and `Pid::new(std::process::id())`; without one it calls `report::open`. |
| `src/ops/menu.rs:102` | Take `MenuCommand::Tasks` out of the `workflows` group. The new arm is `MenuCommand::Tasks => super::tasks::open(app)?`. |
| `src/popup/drives.rs:76` | The `DriveEntry::ProcessList` arm of `select_drive` keeps calling `ops::tasks::open`, which now returns `io::Result<()>`: `Ok` is `Status::Cleared` as today, and a failed report fallback is `Status::Error`, the text `ops::menu::execute` gives the same error. The dialog closes before the row acts, so the viewer stays open. |
| `src/ops/menu/workflows.rs` | Delete the `Tasks` arm, `fn tasks`, and any import left unused (`std::process::Command`, and others if unused). |
| `src/application.rs` | `Redraw` moves here from `wsl_jobs.rs`: it now has two owners, WslJobs and TaskJobs (LSP and vault return `bool`). `LoopState` gains `tasks: task_jobs::TaskJobs`. `pump_background` gains `if self.tasks.pump(app) == Redraw::Needed { self.redraw = Redraw::Needed; }`. `handle_event` sends a paste to `popup::paste_text`. |
| `src/application/wsl_jobs.rs` | `use super::Redraw;` |
| `src/application/timing.rs` | `context()` gains `Overlay::Tasks(_) => "Tasks"`. |
| `src/application/dispatch/dialog_mouse.rs` | `click_special_dialog` gains the `Overlay::Tasks` branch. It returns `true` when a Browse or menu click was handled. It returns `false` when a tracked dialog is open, so buttons take the generic path. |
| `src/application/dispatch/dialog_mouse/button_keys.rs` | `Overlay::Tasks(list) => tasks::tasks_button_key(list, index)`. Confirmation buttons map to `t`/`s`, `k` or Esc. Priority and mask buttons set focus, then Enter. Info and Error map to Esc. |
| `src/bootstrap.rs` | After the clipboard is installed: `if let Some(host) = crate::process_table::system_host() { app.task_host.install(input.process_host(host)); }` |
| `tests/module_boundaries/layers.rs` | The new forbidden paths (4.2) |
| `tests/degenerate_size_test.rs` | `("tasks", Overlay::Tasks(Box::default()))`, plus a populated variant built from `test_support::processes` |
| `tests/tui_bdd/session.rs`, `tests/tui_bdd/mod.rs` | `parse_key` gains `PageUp`, `PageDown`, `Insert` and an `Alt+` prefix. A bare `F` becomes a character (today it panics in the F-key arm). `Session::tasks(left, right, size)` installs a `FixtureHost` in `create_app`, as `Session::inventory` does. The step `the screen eventually contains "…"` sends `Event::FocusGained`, which `handle_event` ignores but which yields a frame, until the text appears or `DEADLINE` passes. |
| `tests/far_manager_hotkeys_test.rs` | A Ctrl+W case using the fixture host (the target already requires `test-helpers`) |

### 4.5 Reused rather than rewritten (DRY)

| Existing piece | Reused for |
|---|---|
| `Cells`, `printable_name`, `head_cells`, `take_cells`, `fit_name`, `clip_name`, `pad_cells`, `right_cells`, `abbreviate_bytes`, `far_block` | Text layout in the table, meters and chrome |
| `visible_window` (moved to `crate::scroll`), `ScrollTop`, `ScrollLeft` | Scrolling |
| `mouse::fkey_widths` / `fkey_at` | Key bar drawing and hit testing |
| `dialog::render_tracked`, `render_warning`, `Button`, `Row` | Nested dialogs |
| `menu_dialog::choices` render and `row_at` | All three menus |
| `MaskPrompt`, `MaskPurpose`, `crate::mask::Mask`, `render_mask_prompt` | Gray `+` and `-` |
| `LineEditor`, `popup::menu_dialog::edit` | Text bars and the priority field |
| `casefold::contains_ignore_case` | Filter and search |
| `Owners` (`getpwuid_r`, cached per uid) | The User column |
| `models::SortOrder` | The sort direction |
| `Viewport`, and the `Viewer::page_for` / `CHROME_ROWS` precedent | Paging |
| The `Redraw` enum | The controller's return value |
| `models::RowCursor` / `RowMotion` (main's list cursor for the sessions, remote and F11 lists) | The table cursor's clamped moves: `ListMotion::with_page` turns a key's move and the page into a `RowMotion`. `ListMotion` stays, because `action::tasks` resolves a key before any page is known. |

The `WslJobs` worker pattern is followed but not merged into a generic type.
This is the second request/response worker, so under the Rule of Three it stays
separate. `task_jobs.rs` carries a `TODO: third worker — generalise with WslJobs`.

---

## 5. Types

Everything below lives in `src/models/tasks/*` unless noted. Fields are private
behind constructors, and each type derives only what its concept supports. All
arithmetic is integer: `u128` intermediates, `checked_*` or `saturating_*`, and
no `as` casts. This matters because release builds set `panic = "abort"` and
`overflow-checks = false` (`Cargo.toml:294-302`). The manifests' reliability
lint set (main `0ab5890`) rejects `as` conversions, indexing, string slicing,
`unwrap`, `expect` and `panic!` in production code.

### 5.1 Identity

| Type | Invariant and API |
|---|---|
| `Pid(NonZeroI32)` | **1..=i32::MAX**. `Pid::new(u32) -> Option<Pid>` rejects 0 and anything above `i32::MAX`. `get() -> u32` (through `unsigned_abs`), `raw() -> i32`. `kill(0)`, `kill(-1)` and `kill(-pgid)` are unrepresentable. rustix's `Pid::from_raw` only `debug_assert`s non-negative input (`rustix-1.1.5/src/pid.rs:43`), so this type is the real guard. Only `platform` converts it. |
| `StartTime(u64)` | An opaque creation stamp in the source's own unit: Linux stat field 22 (ticks since boot), Windows creation FILETIME, ps `lstart` epoch seconds. It derives only `Eq`, `Ord` and `Hash`. |
| `Start` | `Known(StartTime)` or `Unknown`. `Unknown` occurs only when Windows refuses `GetProcessTimes`. |
| `ProcessKey { pid: Pid, start: Start }` | `Copy + Eq + Ord + Hash`. **The one identity.** The cursor, marks, frozen targets, CPU baselines and the command cache all use it. A reused PID is a different key. |
| `TaskSession(u64)` | Minted per open by `TaskHostSlot::next_session()`. A change of session means a fresh worker and empty baselines. |

### 5.2 Units

| Type | Invariant and API |
|---|---|
| `Percent(u32)` | Hundredths of a percent; `FULL = 10_000`. `ratio(part: u128, whole: u128) -> Option<Percent>` returns `None` when `whole == 0`, rounds half up and saturates. `at_most(ceiling)`. CPU% may exceed `FULL`, up to `cores × FULL`. |
| `CpuUsage` | `Pending` (the first sample, a new `ProcessKey`, or a zero delta) or `Measured(Percent)`. A first sample is never a fake `0.0`. |
| `MemoryBytes(u64)` | `from_kib(u64) -> Option` (checked ×1024), `from_pages(u64, page: u64) -> Option` (checked), `bytes()`. Kept apart from `FileSize`, whose Display and semantics are for files. |
| `Load(u32)` | Hundredths. `LoadAverage { one, five, fifteen }`. |
| `ClockTicks(NonZeroU64)`, `PageSize(NonZeroU64)` | In `process_table` only. The conversion factors come from `platform`, and zero is rejected as `SampleError::Malformed`. |

### 5.3 Priority

| Type | Invariant and API |
|---|---|
| `Niceness(i8)` | **−20..=19** through `TryFrom<i32>`. `MIN`, `MAX`, `step(NiceStep) -> Option<Niceness>` (`None` at a bound). |
| `NiceStep` | `Raise` (nice − 1: `]`, and Up in the form) or `Lower` (nice + 1: `[`) |
| `KernelPriority(i32)` | Raw stat field 18. `is_realtime()` is true at −100. |
| `PriorityClass` | `Idle`, `BelowNormal`, `Normal`, `AboveNormal`, `High`, `Realtime`. What Windows **reports**. |
| `SettableClass` | `Idle`, `BelowNormal`, `Normal`, `AboveNormal`, `High`. What the viewer may **set**: Realtime is unrepresentable. `PriorityClass::settable() -> Option<SettableClass>` returns `None` for Realtime, so renice is refused. `step(NiceStep) -> Option<SettableClass>`. |
| `Priority` | `Unix { kernel: KernelPriority, nice: Niceness }`, `Windows { class: PriorityClass }` or `Unknown` |
| `PriorityChange` | `Nice(Niceness)` or `Class(SettableClass)` |

### 5.4 Signals

- **`Signal`**: `Terminate`, `Kill`, `Hangup`, `Interrupt`, `Quit`, `Abort`,
  `Stop`, `Continue`, `TerminalStop`, `User1`, `User2`, `Alarm`, `WindowChange`.
  - These are the POSIX signals that rustix defines on every Unix target. The
    Linux-only `STKFLT` and `PWR` are not offered.
  - `name()` gives `"SIGTERM"`; `purpose()` gives `"ask to terminate"`.
  - `Signal::OFFERED: &[Signal]` is the full list above, in that order (TERM
    first) under `cfg(unix)`, and `&[]` under `cfg(windows)`, where F9 behaves as
    F8.
- **`KillOffer`**: `TerminateOrKill` (F8/Del: Terminate or Kill, Terminate
  focused) or `Send(Signal)` (after the F9 menu).
- `platform` is the only mapper to `rustix::process::Signal`: one exhaustive
  `match`.

### 5.5 Raw sample and ports (`sample.rs`)

```rust
pub struct Sample { pub system: SystemSample, pub processes: Vec<ProcessSample>, pub unreadable: u32 }
pub struct SystemSample {
    pub total: CpuCounters, pub cores: Vec<CoreSample>,           // CoreSample { index: u16, counters }
    pub memory: Option<MemorySample>, pub swap: Option<SwapSample>,
    pub load: Option<LoadAverage>, pub uptime: Option<Duration>, pub running: Option<u32>,
}
pub struct CpuCounters { pub user: Duration, pub nice: Duration, pub system: Duration,
                         pub idle: Duration, pub virtualised: Duration }   // already folded, 6.1.1
pub struct MemorySample { total, free, available, buffers, cached, reclaimable, shared: MemoryBytes }
pub struct SwapSample { total, free, cached: MemoryBytes }
pub struct ProcessSample {
    pub key: ProcessKey, pub parent: Option<Pid>, pub name: String, pub command: String,
    pub owner: Owner, pub state: ProcessState, pub priority: Priority, pub threads: u32,
    pub cpu_time: Duration, pub memory: Option<MemoryUsage>, pub running_for: Option<Duration>,
    pub kind: ProcessKind,
}
pub struct MemoryUsage { pub virtual_size: MemoryBytes, pub resident: MemoryBytes, pub shared: Option<MemoryBytes> }
pub enum Owner { Named(String), Uid(u32), Unknown }
pub enum ProcessState { Running, Sleeping, DiskSleep, Zombie, Stopped, TracingStop, Dead, Idle, Parked, Other(u8) }
pub enum ProcessKind { User, KernelThread }

pub trait ProcessHost {
    fn reader(&self) -> Result<Box<dyn ProcessReader>, SampleError>;
    fn signal(&self, key: ProcessKey, signal: Signal) -> Result<(), ControlError>;
    fn set_priority(&self, key: ProcessKey, change: PriorityChange) -> Result<(), ControlError>;
}
pub trait ProcessReader: Send { fn sample(&mut self) -> Result<Sample, SampleError>; }

pub enum SampleError { Unsupported, Unreadable { part: SystemPart, kind: io::ErrorKind },
                       Malformed(SystemPart), WorkerUnavailable, WorkerStopped, WorkerStalled }
pub enum SystemPart { ProcessList, CpuTimes, ClockRate, PageSize }
pub enum ControlError { Gone, Replaced, PermissionDenied, Critical, Unverifiable, ReadOnly,
                        Refused(ControlRefusal), Unsupported, Failed(io::ErrorKind) }
```

- `ProcessState::from_letter(u8)` is total. `Other` keeps a printable ASCII
  byte, and anything else becomes `b'?'`.
- `ProcessHost` is used on the UI thread only, yet is `Send + Sync`: `App`
  shares it through an `Arc`, and replay's read-only host wraps another. Before
  T19 it needed no `Send`, since `App` is
  already `!Send` (`ClipboardSlot` holds a non-`Send` backend).
- Only the reader moves to the worker, so only `ProcessReader: Send`.

### 5.6 Derived snapshot (`snapshot.rs`, `meters.rs`)

- `Baselines { system: Option<SystemCounters>, cpu_time: HashMap<ProcessKey, Duration> }`.
  This is a lookup table and is never rendered.
- `Snapshot { rows: Vec<ProcessRow>, meters: Meters, cores: usize }`.
  - Rows are unique by `ProcessKey`: the adapter de-duplicates.
  - `ProcessRow { sample: ProcessSample, cpu: CpuUsage, memory_share: Option<Percent> }`.
- `Meters { cpu: CpuMeter, cores: Vec<CpuMeter>, memory: Option<MemoryMeter>, swap: Option<SwapMeter>, tasks: TaskCounts, load: Option<LoadAverage>, uptime: Option<Duration> }`.
- `CpuMeter` is `Pending` or `Measured(CpuBar)`.
  - `CpuBar { user, nice, kernel, virtualised: Percent }`.
  - Invariant: the segments sum to at most `FULL`, because every segment is at
    most the total it is divided by.
  - `busy()` gives the value printed on the bar.
- `MemoryMeter { total, used, buffers, shared, cache: MemoryBytes }`.
  - Invariant: `used + buffers + shared + cache <= total`. The constructor clamps
    the segments in that order.
- `SwapMeter { total, used, cache }`, with `used + cache <= total`.
- `TaskCounts { processes, threads, kernel_threads: u32, running: Option<u32> }`.

### 5.7 View state

| Type | Values and rules |
|---|---|
| `SortKey` | `Pid`, `User`, `Priority`, `Virtual`, `Resident`, `Shared`, `State`, `Cpu`, `CpuTime`, `Started`, `Name`, `Command`. `natural()` is largest first for Cpu, Resident, Virtual, Shared and CpuTime, and ascending for the rest. |
| `Sort` | `{ key: SortKey, order: models::SortOrder }`. `Direct` means the key's natural direction. The default is `{ Cpu, Direct }`. **Ties always break by PID ascending**, so the order is deterministic. Choosing a key leaves tree view (htop manual). |
| `Arrangement` | `Flat` or `Tree` |
| `CommandView` | `CommandLine` or `Name` |
| `KernelThreads` | `Shown` or `Hidden` (the default) |
| `UserScope` | `All` or `Only(Owner)` |
| `FilterTerms(Vec<String>)` | Split on `|`, trimmed, empty terms dropped, lowercased. It matches when any term is contained in the name or the command. An empty list matches everything. |
| `ViewRow` | `{ row: usize, line: TreeLine }` |
| `TreeLine` | `Flat`, or `Node { depth: u16, sibling: Sibling, open: u64 }`. `Sibling` is `Middle` or `Last`. `open` is a bitmask of ancestor levels that still have siblings below; levels deeper than 64 are drawn without guides. |
| `Cadence` | `Live` or `Held` |
| `Schedule` | `Due`, `At(Instant)`, `Waiting { since: Instant, request: Request }` or `Held` |
| `Request` | `Automatic` or `Explicit` |
| `Health` | `Fresh`, `Overdue`, `Stalled` or `Failed(SampleError)` |
| `Table` | `Waiting`, `Ready { snapshot, health }` or `Failed(SampleError)`. `Failed` is used only while there is no snapshot. |
| `Modal` | `Browse`, `Help(ScrollTop)`, `Info(ProcessKey)`, `Filter(TextBar)`, `Search(SearchBar)`, `PidJump(PidQuery)`, `Menu(TaskMenu)`, `Mask(MaskPrompt)`, `Confirm(KillConfirm)`, `Priority(PriorityForm)`, `Renice(NiceConfirm)` (the confirmation `[` asks for), `Error(Vec<Failure>)`. One enum, so two nested dialogs can never be open at once. The whole thing lives inside `TaskList`, so help and confirmations never replace the view and its worker. |
| `TaskMenu` | `Sort(MenuCursor)`, `Signal { targets: Vec<Target>, skipped: Vec<Skipped>, cursor: MenuCursor }`, `User { owners: Vec<Owner>, cursor: MenuCursor }` |
| `MenuCursor(usize)` | Always less than its menu's length; the menu's constructor and `navigate` keep it so. |
| `KillConfirm` | `{ targets: Vec<Target>, skipped: Vec<Skipped>, offer: KillOffer, focus: usize }`, with `focus` less than the button count |
| `PriorityForm` | `{ targets: Vec<Target>, value: LineEditor, focus: FormFocus }`, with `FormFocus` one of `Field`, `Set`, `Cancel` |
| `PidQuery(String)` | ASCII digits only, at most 10 |
| `SearchBar` | `{ line: LineEditor, found: Found }`, with `Found` `Match` or `NoMatch` |
| `Target` | `{ key: ProcessKey, name: String, owner: Owner, relation: Relation }`, frozen when a dialog opens; the owner is what "belongs to another user (root)" names |
| `Relation` | `Itself`, `Ancestor` or `Other`. Computed by walking `parent` from the own PID through the snapshot, with a visited set, so an untrusted cycle cannot loop. |
| `Refusal` | `OwnProcess`, `Init`, `KernelThread`, `Unverifiable`, `AtLimit(Niceness)`, `RealtimeClass`, `ClassAtLimit`, `UnknownPriority`, `NoTarget` (as built: `ControlRefusal`) |
| `Skipped` | `{ target: Target, refusal: Refusal }` |
| `Notice` | `Sent { signal, count }`, `Reniced { count, nice }` (the Priority form), `Stepped { step, count }` (`]` and `[`), `Refused(Refusal)`, `Marked { changed, total }`. Each renders as one fixed English sentence (section 9); a failure is a `Failure { target, attempt, error }` line of the Error dialog. |
| `NicePlan` | A renice planned before any system call: `Renice::Step(NiceStep)` or `Renice::Set(Niceness)`, one `PlannedChange { target, change, direction }` per target, and the targets `Skipped`. `TaskList::plan_step` and `plan_set` build it. |
| `TaskHostSlot` | `{ host: Option<Arc<dyn ProcessHost>>, sessions: u64 }`, with `install`, `host`, `next_session` |
| `TaskClick` (in `ui`) | `Row(usize)`, `Title(SortKey)`, `Key(u8)`, `MenuLine(usize)` |

No bool or string anywhere above carries domain meaning.

---

## 6. Data sources

### 6.1 Linux (first release)

`ProcFs` reads with `std::fs` only. Each read goes through
`File::open(path)?.take(cap).read_to_end(&mut buf)` into one buffer reused across
a sample, then is decoded lossily. The caps are:

| Files | Cap |
|---|---|
| `stat` | 4 KiB |
| `statm` | 1 KiB |
| `status` | 16 KiB |
| `cmdline` | 4 KiB |
| System files (`/proc/stat`, `/proc/meminfo`, `/proc/loadavg`, `/proc/uptime`) | 1 MiB (`procs_running` comes after the long `intr` line in `/proc/stat`) |

`root` is `/proc` in production and a `TempDir` in tests. `ticks` comes from
`platform::clock_ticks_per_second()` and `page` from `platform::page_size()`.
Both are queried, never hard-coded: arm64 kernels use 16 or 64 KiB pages.

The kernel ends every line it writes with a newline, so the line-oriented
parsers (`statm`, `status`, `/proc/stat`, `/proc/meminfo`, `/proc/loadavg`,
`/proc/uptime`) read only up to the last one: a number the cap cut is left
unread, never read as a smaller one. `stat` needs its fields through the 23rd
instead (6.1.2), and `cmdline`, which has no lines, is shown as far as it was
read.

#### 6.1.1 System files, read once per sample

**`/proc/stat`** is required.

- Lines `cpu …` (the aggregate) and `cpuN …` (one per online core; N keys the
  core, because offline CPUs have no line). The fields are ticks:
  `user nice system idle iowait irq softirq steal guest guest_nice`.
  - Missing trailing fields count as 0, since older kernels print fewer.
  - Fewer than 4 numbers, or anything non-numeric, is
    `SampleError::Malformed(CpuTimes)`.
- They are folded into `CpuCounters`, with saturating subtraction:
  ```
  user'        = user   − guest          // guest is already inside user
  nice'        = nice   − guest_nice     // guest_nice is already inside nice
  system'      = system + irq + softirq
  idle'        = idle   + iowait
  virtualised' = steal  + guest + guest_nice
  ```
  Each value is converted to a `Duration` as `ticks × 1e9 / hz` in `u128`.
- `procs_running R` gives the running count.

**`/proc/meminfo`** is optional; unreadable means no Mem or Swp meter and `-` for
MEM%.

- The format is `Key:   <value> kB`, and `kB` means KiB, converted with a
  checked ×1024.
- Keys read: `MemTotal` (without it there is no memory sample), `MemFree`,
  `MemAvailable`, `Buffers`, `Cached`, `SReclaimable`, `Shmem`, `SwapTotal`,
  `SwapFree`, `SwapCached`.
- A missing optional key counts as 0. A missing `MemAvailable` (kernels before
  3.14) falls back to `MemFree + Buffers + Cached`.

**`/proc/loadavg`** is optional. It holds `a.bb c.dd e.ff r/t last`. Each of the
three loads is parsed as integer hundredths (`int × 100` plus the first two
fraction digits), with no floats.

**`/proc/uptime`** is optional. Its first field, `secs.cc`, becomes a `Duration`.

#### 6.1.2 Per process

The reader lists every entry of `read_dir(root)` whose name parses as a `Pid`.
`self`, `thread-self` and `sys` are ignored.

- **`stat`** is required for the row.
  - Split at the **last** `)`: `comm` may contain spaces and parentheses. Live
    examples are `((sd-pam))`, `npm exec vite p` and `next-server (v1`.
  - The head is `pid (comm`. The pid must equal the directory name, or the row is
    `Malformed`.
  - For the tail tokens, proc(5) field *k* is `tail[k − 3]`. At least 21 tokens
    are required.

  | Field | Meaning | Use |
  |---|---|---|
  | 3 | state | `ProcessState` |
  | 4 | ppid | `parent`; 0 means none |
  | 9 | flags | `PF_KTHREAD = 0x0020_0000` means `KernelThread`. Checked live: kthreadd has 2129984 = 0x208040, with the bit. |
  | 14, 15 | utime, stime | `cpu_time` |
  | 18 | priority | `KernelPriority` |
  | 19 | nice | `Niceness`; out of range is `Malformed` |
  | 20 | num_threads | `threads` |
  | 22 | starttime | `StartTime` and `running_for = uptime − starttime / hz` (saturating) |
- **`statm`**: `size resident shared text lib data dt`, in pages.
  - VIRT = size × page, RES = resident × page, SHR = shared × page, each a
    checked multiply. An overflow makes `memory` `None`.
  - proc(5) calls these values inaccurate, but they are htop's source, and one
    source is used consistently. Field 23 of `stat` is not used for VIRT.
- **`status`**: the line `Uid:\t<real>\t<effective>\t<saved>\t<fs>` gives the
  **effective** uid.
  - This is not the owner of `/proc/<pid>`, which becomes root for
    non-dumpable processes.
  - `Owners::name_of_uid` turns it into a name. The `getpwuid_r` cache lives as
    long as the reader, so NSS or LDAP is asked once per uid per session.
  - The cache keeps a failed lookup (EIO, EMFILE, an LDAP outage) apart from a
    uid with no entry. The reader calls `Owners::forget_failed_lookups()` once
    per sample, so a failure is asked again on the next sample, while a name or
    a missing entry is kept for the session.
  - A uid without an entry becomes `Owner::Uid(n)`, shown as the number.
- **`cmdline`**: NUL-separated. The trailing empty piece is dropped, the
  arguments are joined with spaces and decoded lossily. An empty cmdline (kernel
  threads, zombies) shows as `[comm]`, as ps prints it.
  - **Command cache.** The reader keeps `CommandCache: HashMap<ProcessKey, (comm, command)>`:
    1. A process whose key and comm match the cache reuses its command. This is
       htop's default, "Update process names on every refresh" off, whose option
       text was confirmed in the binary.
    2. A changed comm, which is what `execve` produces, re-reads the file.
    3. A process in state `D` is never read while uncached. It shows `[comm]`
       until it leaves that state, because reading cmdline takes the target's
       mmap lock.
    4. Entries for vanished keys are dropped every sample.
- `environ` and `io` are never read.

**Per-process errors.**

| Condition | Result |
|---|---|
| `NotFound`, or raw OS error `ESRCH` (the Linux value 3, a named const) on any per-process file | The process exited during the scan and is skipped silently. |
| `stat` unreadable for another reason (for example `EACCES` under `hidepid=1`) or malformed | The process is skipped and `unreadable += 1`. |
| `statm` unreadable | `memory: None` |
| `status` unreadable | `Owner::Unknown` |
| `cmdline` unreadable | `[comm]` |

**Whole-sample errors.** An unreadable `root` (`Unreadable { ProcessList }`) or
a missing, unreadable or malformed `/proc/stat` fails the sample.

#### 6.1.3 Kill and renice (`procfs/control.rs`)

`ProcessControl { root, own, ticks }` holds what both need: `/proc` (a
`TempDir` in tests), FFM's own PID and the clock rate the re-read parses
with. `ProcessControl::system()` is `None` should the kernel report a PID or
a clock rate no process can have, and then `system_host()` is `None` too.

**Signal**, `ProcessControl::signal(key, signal)`:

1. `Start::Unknown` gives `Err(Unverifiable)`. FFM itself and PID 1 give
   `Err(Refused(OwnProcess))` and `Err(Refused(Init))`
   (`ControlRefusal::signal_to_pid`), before any system call. The models
   refuse them when the dialog freezes its targets (S3); the host refuses
   them again, so no caller of `ProcessHost::signal` can reach them.
2. Pin the process: `platform::ProcessHandle::open(pid)`, which is `pidfd_open`.
   `classify_open` maps the result:

   | Result | Meaning |
   |---|---|
   | `Ok(h)` | `Pinned(h)` |
   | `ENOSYS` (kernel before 5.3) or `EPERM` (a seccomp filter) | `Unpinned` |
   | `ESRCH` | `Gone` |
   | anything else | `Failed(kind)` |
3. Read `root/<pid>/stat` with the same parser. `verify_start(result, expected)`:

   | Read result | Outcome |
   |---|---|
   | `NotFound` / `ESRCH` | `Gone` |
   | Another read error | `control_error` (below) |
   | A line that does not parse | `Unverifiable` |
   | A different `StartTime` | `Replaced` |
   | Equal | continue |

   A kernel thread, by the flags just re-read, gives `Err(Refused(KernelThread))`
   (`ControlRefusal::signal_to_kind`).
4. Send:
   - `Pinned(h)`: `h.signal(signal)` (`pidfd_send_signal`). A process that dies
     after step 3 still owns the pidfd, so the kernel answers `ESRCH` instead of
     signalling a newcomer.
   - `Unpinned`: `platform::signal_process(pid, signal)` (`kill`). The window
     between step 3 and the call is documented.

   `control_error(io::Error)` maps the errno:

   | errno | `ControlError` |
   |---|---|
   | `ESRCH` | `Gone` |
   | `EPERM`, `EACCES` | `PermissionDenied` |
   | anything else | `Failed(kind)` |

**Renice**, `ProcessControl::set_priority(key, Nice(n))`: an unknown start is
`Unverifiable`, `Class(_)` is `Unsupported` on Linux, and FFM itself, init and
kernel threads may be reniced (S4). The same start-time check, then
`platform::set_niceness(pid, n)` (`setpriority(PRIO_PROCESS, Some(pid))`). There
is no pidfd form, so a small window remains; the worst case is one nice step on a
newcomer. On Linux this renices only the thread whose TID equals the PID, as
`renice(1)` does. The help page and the parity doc say so.

The errno values are named consts with Linux values, so the pure mapping
compiles and is tested on every CI OS.

#### 6.1.4 Platform wrappers (`src/platform/unix.rs`, safe rustix, not allowlisted)

| Wrapper | cfg | Implementation |
|---|---|---|
| `page_size() -> u64` | `unix` | `rustix::param::page_size` |
| `clock_ticks_per_second() -> u64` | `unix` | `rustix::param::clock_ticks_per_second` |
| `signal_process(Pid, Signal) -> io::Result<()>` | `unix` | `kill_process` |
| `set_niceness(Pid, Niceness) -> io::Result<()>` | `unix` | `setpriority_process`, always `Some(pid)`; `None` would mean the caller |
| `ProcessHandle::open(Pid)`, `ProcessHandle::signal(Signal)` | `target_os = "linux"` | `pidfd_open`, `pidfd_send_signal` |

- One private `fn rustix_pid(Pid) -> io::Result<rustix::process::Pid>` does the
  only conversion.
- One exhaustive `match` maps `Signal` to `rustix::process::Signal`.
- `pidfd` is gated `cfg(target_os = "linux")` in rustix 1.1.5, so the handle has
  the same gate.

### 6.2 Derived values (models; every platform)

`Snapshot::derive(previous: &Baselines, sample: Sample)` computes everything
below. `Δ` is `now.saturating_sub(previous)` per folded field.

```
Δtotal     = Δuser' + Δnice' + Δsystem' + Δidle' + Δvirtualised'     // per CPU line
bar        = user Δuser'/Δtotal, nice Δnice'/Δtotal, kernel Δsystem'/Δtotal,
             virtualised Δvirtualised'/Δtotal                         // Percent::ratio each
busy       = (Δtotal − Δidle') / Δtotal
Δtotal = 0 or no previous → CpuMeter::Pending

ncpu       = max(1, sample.system.cores.len())
CPU%       = Percent::ratio(Δcpu_time × ncpu, Δtotal(aggregate "cpu" line)), at most ncpu × FULL
             (Δtotal / ncpu is the elapsed time per CPU: htop's "Irix mode", where one busy core = 100%)
             Pending when the ProcessKey has no baseline (a first sample, or a reused PID with a
             new start time) or Δtotal = 0
MEM%       = Percent::ratio(RES, MemTotal), at most FULL; None without memory
TIME+      = cpu_time
Tasks      = processes: rows of kind User; threads: Σ threads(User rows) − processes;
             kthr: rows of kind KernelThread; running: procs_running
Mem meter  (htop 3.x formula, (knowledge) of its LinuxMachine.c):
             cache   = Cached + SReclaimable − Shmem                        (saturating)
             used    = Total − (Free + Cached + SReclaimable + Buffers)     when that sum ≤ Total,
                       else Total − Free
             segments used, buffers (Buffers), shared (Shmem), cache — clamped to Total in that order;
             text "used/total"
Swp meter  used = SwapTotal − SwapFree − SwapCached (saturating), cache = SwapCached; text "used/total"
```

Worked example from this host's `/proc/meminfo`:

| Key | Value |
|---|---|
| MemTotal | 131636496 kB |
| MemFree | 25897000 kB |
| Buffers | 6258456 kB |
| Cached | 52449252 kB |
| SReclaimable | 6932888 kB |
| Shmem | 225196 kB |

Then used = 131636496 − 91537596 = 40098900 kB (38.2 GiB), and cache =
59156944 kB. For comparison, procps `free`'s used, `Total − MemAvailable`, is
41940896 kB (40.0 GiB). The formula is one pure function, so switching to the
`free` definition is a one-line change (open question 4).

### 6.3 Windows (phase W; not in the first release)

All FFI goes in the new `src/platform/windows/processes.rs`. It is added to
`BOUNDARIES` in `tests/unsafe_boundary_test.rs` with the reason "Win32 process
enumeration, times, memory, termination and priority". It follows the
`platform/windows.rs` pattern:

- `#[allow(unsafe_code)]` per item;
- a `// SAFETY:` comment on every block;
- handles wrapped in `OwnedHandle`;
- failures reported through `io::Error::last_os_error()`.

`Cargo.toml` adds the windows-sys 0.61.2 features
`Win32_System_Diagnostics_ToolHelp` (manifest line 200) and
`Win32_System_ProcessStatus` (line 232). No crate is added.

`OpenProcess`, `GetProcessTimes`, `GetSystemTimes`, `TerminateProcess`,
`Get/SetPriorityClass`, `QueryFullProcessImageNameW`, `OpenProcessToken`,
`IsProcessCritical` and `GetActiveProcessorCount` are all under
`Win32_System_Threading`, which is already enabled. `GetTokenInformation` and
`LookupAccountSidW` are under `Win32_Security`. `GlobalMemoryStatusEx` and
`GetTickCount64` are under `Win32_System_SystemInformation`.

**Enumeration.**

- `CreateToolhelp32Snapshot` with `Process32FirstW`/`NextW` gives pid, ppid,
  threads and the exe name. PID 0 (Idle) is skipped. PID 4 (System) is
  `KernelThread`.
- For each process, `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)`, then:

  | Call | Gives |
  |---|---|
  | `GetProcessTimes` | creation time as `Start::Known`, kernel + user time as `cpu_time` |
  | `K32GetProcessMemoryInfo` | `WorkingSetSize` as RES, `PrivateUsage` as VIRT; SHR is `None` |
  | `GetPriorityClass` | `Priority::Windows` |
  | `QueryFullProcessImageNameW` | the Command column |
  | Token user and `LookupAccountSidW` | the owner, cached by SID |

  Access denied leaves fields `None` or `Unknown`, and the start as
  `Start::Unknown`, which refuses control as `Unverifiable`.
- **System values.** `GetSystemTimes` gives total CPU only, with
  kernel − idle as system and no per-core bars. `GlobalMemoryStatusEx` gives Mem;
  there is no Swp or load average. `GetTickCount64` gives uptime.

**Kill.**

1. `OpenProcess(PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION)`. The
   handle pins the object.
2. The creation time must equal the key's start.
3. `IsProcessCritical` true gives `ControlError::Critical`, because ending a
   critical process stops the system.
4. `TerminateProcess(h, 1)`.

There are no signals: F8 offers `{Terminate} [Cancel]`, and F9 does what F8 does.

**Priority.** Same pinning and check, then `SetPriorityClass` with a
`SettableClass`. F7 shows a Radio row of settable classes. A Realtime process is
refused (`RealtimeClass`).

**Tests.** The pure conversions (FILETIME to `Duration`, raw class to
`PriorityClass`, `GetSystemTimes` deltas) compile under `cfg(any(windows, test))`,
so the Linux coverage and mutants jobs reach them.

When phase W lands, `system_host()` returns `Some` on Windows and the tasklist
branch of `report_command()` goes.

### 6.4 macOS and BSD (phase M; not in the first release)

**Reading.** `ps -axww -o pid=,ppid=,uid=,nice=,pri=,vsz=,rss=,time=,state=,lstart=,args=`
runs through an injected runner. The system runner is
`ops::menu::process::capture`: a one-minute timeout and an 8 MiB output cap,
after which it kills the command and everything the command started (main
`0125c57`). Parsing:

- Split on whitespace.
- The first nine fields are single tokens.
- `lstart` is **exactly five tokens** (`Wed Sep 23 10:00:00 2026`) and becomes
  `StartTime` in epoch seconds via chrono, which is already a dependency.
- `args` is the rest of the line.

The parser compiles under `cfg(any(all(unix, not(target_os = "linux")), test))`,
so Linux CI tests it with fixture text.

**Derived values.** CPU% comes from deltas of `time`. Load and uptime come from
`sysctl`. The CPU, Mem and Swp meters are omitted.

**Kill and renice** use the rustix wrappers after an `lstart` re-check, with no
pidfd; the window is documented.

macOS has no CI job, so this phase must be verified by hand on a Mac
(open question 5).

---

## 7. Refresh model

**Owner.** `TaskJobs` in `src/application/task_jobs.rs`, a `LoopState` field:

```rust
pub(super) struct TaskJobs { sampler: Sampler, logged: Logged }
enum Sampler { Idle, Running(Worker), Abandoned { handle: JoinHandle<()>, since: Instant } }
struct Worker { session: TaskSession, requests: SyncSender<()>,
                results: Receiver<Result<Sample, SampleError>>, handle: JoinHandle<()>, flight: Flight }
enum Flight { Idle, Since(Instant) }
```

A worker and an abandoned thread are variants of one `Sampler`, so "no worker
starts beside an abandoned thread" is a fact of the type rather than a check.
`Flight` says whether the worker owes an answer, which decides how it is
retired once the list, and with it the model's schedule, is gone.

`pump(app) -> Redraw` wraps the private `pump_at(app, now)`, the
`ops::vault::maintain_secrets(app, now)` pattern, so tests drive time with
synthetic `Instant`s.

**Constants.** All four live once, in `models/tasks/schedule.rs`.

| Constant | Value | Why |
|---|---|---|
| `REFRESH_INTERVAL` | 1.5 s | htop's default delay of 15 tenths *(knowledge; the manual gives only the 1..100 clamp)* |
| `WARM_UP` | 500 ms | Delay after a session's first sample. At CLK_TCK 100 that gives 2 % resolution per process. |
| `OVERDUE_AFTER` | 5 s | A request in flight this long marks the view `Overdue` (title ` · stale `) |
| `STALL_LIMIT` | 10 s | A request in flight this long is abandoned (`Stalled`, title ` · not responding `) |

**`pump_at(app, now)` runs on every loop pass.** The live reader wakes at least
every 100 ms (`recording.rs:15`); on Windows, and on Unix without a tty, it may not
wait at all (`recording/tty.rs`). So a pass costs one `match`, one `try_recv` and
one `Instant` comparison, never I/O. In order:

1. **The viewer is closed.** If `app.overlay` is not `Tasks`, retire the worker
   and return `Unchanged`.
   - An idle worker is dropped; its thread ends at its next `recv`.
   - A worker with a request in flight becomes `Abandoned`, with the time
     that request was sent.
   - No thread exists while the viewer is closed, so
     `application/tests/idle_render.rs` keeps its exact draw counts.
2. **The session changed.** If the worker's session differs from
   `list.session()`, retire it the same way.
3. **Drain.** `try_recv` the results. On a result:
   - `list.apply_sample(result, now)` (models).
   - Record `"Background/Tasks/Sample"` with the round-trip time in
     `app.performance`.
   - An error is logged once per session and error kind.
   - Return `Needed`.

   On `Disconnected`: `list.worker_lost(SampleError::WorkerStopped)` and drop the
   worker. There is no respawn loop; reopening starts a new session.
4. **Watch the request in flight** (`now − since`, where `since` is from
   `Schedule::Waiting`):
   - At `OVERDUE_AFTER` or later: `list.mark(Health::Overdue)`, once.
   - At `STALL_LIMIT` or later: move the handle to `abandoned`, call
     `list.mark(Health::Stalled)` (the schedule goes back to `Due`), and return
     `Needed`. With no snapshot yet there is no health to mark, so the table
     becomes `Failed(WorkerStalled)` instead: a first sample stuck in the
     kernel must not look like one still starting.
   - Behind an `Abandoned` thread still running, a list that has no table yet
     (reopened while the last session's read is stuck) becomes
     `Failed(WorkerStalled)` once that read is `STALL_LIMIT` old: no thread
     can start for it until the stuck one ends, and it must not look as if it
     were starting either.
5. **Spawn.** If the sampler is `Idle`, or `Abandoned` and `is_finished()`,
   and the list's schedule is not `Stopped` (its worker was lost this
   session):
   - Clear the finished abandoned thread.
   - Get `app.task_host.host()?.reader()`. With no host the list's worker is
     lost as `Unsupported`; a reader that cannot open is lost with its own
     error (for example `Malformed(ClockRate)`). Either is shown once.
   - Spawn the thread `ffm-tasks` (`std::thread::Builder::name`). A spawn error
     is logged and loses the worker as `WorkerUnavailable`.

   "Lost" is `TaskList::worker_lost`: the error is shown and the schedule
   becomes `Stopped`, so nothing is tried again until the list is reopened.
6. **Request.** If `list.take_request(now)` answers yes, `try_send(())`. A
   disconnected channel loses the worker as `WorkerStopped`.

The rules that follow from this:

- At most one sampling thread is alive, plus at most one abandoned thread.
  Abandoning happens only when no thread is already abandoned.
- Nothing is ever joined. Closing the viewer or quitting ffm
  (`LoopState` dropped at the end of `run_loop`) never waits on a read stuck in
  the kernel. A detached thread ends at its next channel operation, or with the
  process.

**Worker loop.**

```rust
while requests.recv().is_ok() {
    if results.send(reader.sample()).is_err() { break }
}
```

Both channels are `sync_channel(1)`, and there is only ever one request in
flight, so a slow sample can never build a queue.

**Schedule transitions** (model, `schedule.rs`):

| Event | From | To |
|---|---|---|
| open | — | `Due`, with cadence `Live` |
| `take_request(now)` | `Due` | `Waiting { now, Explicit }` |
| `take_request(now)` | `At(t)`, `now >= t` | `Waiting { now, Automatic }` |
| result applied | `Waiting` | `At(now + WARM_UP)` after the session's first snapshot, then `At(now + REFRESH_INTERVAL)`, when `Live`; `Held` when `Held`. The interval runs from completion, so a slow sample stretches the period instead of queueing reads. |
| result arrives while `Held` and the request was `Automatic` | `Waiting` | `Held`; the result is discarded, so the screen really holds |
| F11 / `Z` (Live → Held) | `At(_)` | `Held` (a `Waiting` request stays; its result is discarded if it was `Automatic`. A `Due` one stays too: it was asked for, so it is taken and shown, then `Held`) |
| F11 / `Z` (Held → Live) | `Held` | `Due` |
| Ctrl+R / Ctrl+L | anything but `Waiting` | `Due` (even while held: one sample, then `Held`) |
| Ctrl+R / Ctrl+L | `Waiting { .. }` | `Waiting { request: Explicit }` |
| delivery or renice finished | any | `Due`, so the result shows at once |
| stalled | `Waiting` | `Due` (served when a thread may spawn again); `Held` when held and the request was `Automatic`, whose answer would have been discarded |

**Applying a sample** (`TaskList::apply_sample`):

1. `Snapshot::derive` runs with the previous baselines.
2. The view is rebuilt: filter, user scope, kernel threads, then sort or tree.
3. Marks whose key vanished are pruned.
4. The cursor finds its `ProcessKey` again. If that process is gone, the cursor
   keeps its index, clamped to the list, and takes that row's key.
5. Health becomes `Fresh`.
6. An open `Menu`, `Confirm` or `Priority` dialog keeps its frozen targets
   (section 8).

**Redraw.** `Needed` is returned only when something changed: a result, a failure
or a health mark. The minute-change redraw and input-driven redraws are
unchanged. An open, idle viewer therefore draws about once per 1.5 s, and a
closed one never.

---

## 8. Safety rules for kill and renice

| # | Rule |
|---|---|
| S1 | **Targets are identities, frozen when the dialog opens.** F8, Del, F9 and `k` freeze `Vec<Target>`: the marked processes in view order, or else the cursor process. Refreshes continue behind the dialog but can never change what is signalled. The confirmation always names the targets (`preview_lines`). |
| S2 | **Every signal is confirmed.** It takes a red Warning dialog plus Enter or the button. Terminate (SIGTERM) is the focused default; Kill needs Tab, Right or `k`. The F9 path is menu, then confirmation. A click can only open these dialogs. Far's Del, which kills at once in Far, opens the confirmation instead. The confirmation does not depend on Options > Confirmations (open question 3). |
| S3 | **Refusals before any syscall** (`targets::refusal`, pure). The `Refusal` values are listed after this table. Refused targets are listed as `Skipped` in the dialog, and the rest still go. With nothing left, only a notice is shown. |
| S4 | **Renice limits.** `Niceness` cannot leave −20..=19. A step past a bound is `AtLimit` with no syscall. On Windows `SettableClass` cannot express Realtime. Renicing FFM itself is allowed (harmless). `]` acts at once, as in htop; `[` does for root and asks first for anyone else, who cannot raise the priority again (resolved question 6). After a successful renice, `TaskList::record_nice` stores the accepted value, so the next `]` steps from what the kernel accepted, not from a stale snapshot. |
| S5 | **PID validity.** `Pid` is 1..=i32::MAX, and only `platform` builds a rustix `Pid`, from it. `kill(0)`, `kill(-1)` and group kills cannot be expressed, and nothing calls `kill_process_group` for this feature. |
| S6 | **PID reuse.** Linux: pidfd first, then the start-time check, then `pidfd_send_signal` (6.1.3). A pidfd opened before the check proves identity: two processes sharing a PID within one clock tick is the only exception. Without a pidfd (kernel before 5.3, seccomp) and for renice, a check-then-act window of microseconds remains; it is documented. Windows (phase W): the handle pins the object. |
| S7 | **Ancestors.** `Relation::Ancestor` targets (the shell, tmux, sshd, the terminal) may be signalled, but the confirmation adds `FFM runs inside <pid> <name>; ending it ends this session.` If the terminal goes, the session ends through the existing `TerminalLost` path. |
| S8 | **Replay is read-only.** Replaying a recording that contains F8 Enter would otherwise signal whatever process happens to be under the cursor on the replaying machine. `recording/processes.rs` wraps the host so `signal` and `set_priority` return `ControlError::ReadOnly` during replay. |
| S9 | **Untrusted text.** comm, cmdline, user names and Windows image paths pass through `printable_name`, which turns control and bidi characters into U+FFFD, before they are measured and drawn. `environ` and `io` are never read. Command lines appear only on screen: logs and error texts use the PID and comm, never argv, which may hold secrets. `TaskList` and `Snapshot` derive no `Debug`. |
| S10 | **Tests never signal a stranger.** BDD, ops and popup tests use `FixtureHost`, which only records. Real-kernel tests signal only `sleep` children they spawned themselves. |
| S11 | **Permissions are typed outcomes.** `EPERM` and `EACCES` become `PermissionDenied`. The message distinguishes "belongs to another user" (signal, or nice up) from "raising priority needs root (CAP_SYS_NICE)" (nice down). |
| S12 | **FFM's own resilience.** The sampler is never joined, and kill and renice do not depend on it. A hung `/proc` read can freeze neither the UI nor quitting, and the user can still kill the process that hangs the reads. |

The `Refusal` values for S3 are:

- **`OwnProcess`**, when the PID is `std::process::id()`: TERM would leave the
  terminal raw, and STOP would freeze the UI. The notice says to quit with F10.
- **`Init`**, for PID 1: SIGINT to systemd as PID 1 starts ctrl-alt-del (a
  reboot). Inside a container, PID 1 is the container itself.
- **`KernelThread`**, which includes Windows PID 4: kernel threads ignore
  signals.
- **`Unverifiable`**, when the start is `Start::Unknown`.
- **`NoTarget`**, when nothing is selected.

---

## 9. Error handling

| Source | Behaviour | Text (fixed English sentence) |
|---|---|---|
| A whole sample fails and there is no snapshot yet | `Table::Failed(e)`; the body shows the sentence; the next attempt runs at the normal interval | `The process table cannot be read (/proc: permission denied).` |
| A whole sample fails and a snapshot exists | The last snapshot stays; `Health::Failed(e)`; title ` · refresh failed `; retried at the normal interval; logged once per session and error kind with `diagnostics::log_error` | — |
| A process vanished during the scan | Skipped silently | — |
| A process is unreadable or malformed | Skipped and counted; footer `, 3 unreadable` | — |
| An optional per-process file | Degrades that field only (6.1.2) | — |
| Worker spawn fails | `Failed(WorkerUnavailable)`, logged | `The process reader could not start.` |
| The worker disconnects | `Failed(WorkerStopped)`; no respawn until reopened | `The process reader stopped.` |
| A request is in flight for 5 s or more | `Overdue`, title ` · stale `, last snapshot kept | — |
| A request is in flight for 10 s or more | `Stalled`, title ` · not responding `, the thread abandoned (7) | — |
| The same, before the first snapshot | `Table::Failed(WorkerStalled)`; the body shows the sentence until a sample arrives | `The process reader is not responding.` |
| `ControlError::Gone` | Error dialog line | `4242 firefox has already exited; nothing was sent.` |
| `Replaced` | ″ | `PID 4242 now belongs to another process; nothing was sent.` |
| `PermissionDenied` (signal, or nice up) | ″ | `4242 firefox belongs to another user (root).` |
| `PermissionDenied` (nice down) | ″ | `Only root can raise a process's priority (CAP_SYS_NICE).` |
| `Critical` (Windows) | ″ | `4242 is a critical system process; ending it stops Windows.` |
| `Unverifiable` | ″ | `FFM cannot confirm which process 4242 is; nothing was sent.` |
| `ReadOnly` | ″ | `This is a replayed session; processes are not signalled.` |
| `Refused(refusal)` (the host's own refusal, 6.1.3) | ″ | The refusal's sentence, as in the Refusals row |
| `Unsupported` | ″ | `Not supported on this platform.` |
| `Failed(kind)` | ″, detail to the log (PID and comm only) | `Could not signal 4242 firefox: <kind>.` |
| Refusals | Notice, or a `Skipped` line in the dialog | `This is FFM itself; quit with F10.` · `PID 1 (init) is never signalled from here.` · `Kernel threads ignore signals.` · `Nice is already 19.` · `The Realtime class is not changed from here.` · `No process is selected.` |
| All deliveries succeed | Notice | `Sent SIGTERM to 2 processes.` · `Nice set to 5 for 1 process.` |
| The report fallback (no host) fails | Today's `ops::menu` error path, unchanged | — |

Callers branch on enum variants, never on message text. The render path never
panics at any size. The parsers never index or unwrap: they use `split_once`,
`get`, `checked_*` and `parse::<T>().ok()`. That is enforced by the manifests'
reliability lint set and by the robustness corpus (10.2), not by module-level
lint attributes (decision D13).

---

## 10. Test strategy

**General rules.**

- Tests are sentences, in sibling `tests*.rs` files, table-driven where there
  are cases.
- Builders come from `models/tasks/tests_support.rs` and `test_support::processes`.
- Fixture text uses only `bob`, `alice`, `/home/bob`, `/usr/bin` and
  `/srv/x`, to satisfy `tests/private_data_lint_test.rs`.
- Tests use `App::with_panels(Panel::from_entries(…))`, never `App::new()`.

### 10.1 Parsers (`process_table/procfs/tests_parse.rs`)

These run on the Windows CI too, through `cfg(any(linux, test))`.

- **stat.**
  - comm values `((sd-pam))`, `npm exec vite p`, `next-server (v1`, a `\n`
    inside comm, and non-UTF-8 bytes.
  - `ppid` 0; flags `0x208040` (a kernel thread) against `0x400100` (not one).
  - Truncation after every field gives `Malformed`.
  - A pid that differs from the directory is rejected.
  - Nice −21 and 20 are rejected; −20 and 19 are accepted.
  - A pid above `i32::MAX` is rejected.
- **statm.** Page multiplication, and overflow gives `None`.
- **status.** `Uid:` with 4 fields picks the second; with 1 field the owner is
  `Unknown`.
- **cmdline.** NUL joining, the trailing NUL, empty becomes `[comm]`, and the
  4 KiB cap.
- **/proc/stat.** 4, 8 and 10 cpu fields; guest and guest_nice folding; no
  `cpuN` lines; `procs_running` after a very long `intr` line.
- **meminfo.** Keys in any order; without `MemAvailable`; without `MemTotal`.
- **loadavg, uptime.** Exact integer results.

### 10.2 Robustness corpus (`tests_robustness.rs`)

Every parser is fed:

- every prefix of every fixture;
- 10 000 byte flips from a fixed-seed LCG, with no new dev-dependency.

The assertion is `Ok` or `Err`, never a panic. This protects the release profile
(abort on panic, no overflow checks). A file read for one value (`statm`,
`status`, `/proc/loadavg`, `/proc/uptime`) must also read every proper prefix
as nothing or as the whole value, never as a smaller number.

### 10.3 Reader over a fake procfs (`tests_reader.rs`, `TempDir` root)

Fixture files end their lines with a newline, as the kernel writes them (6.1).

- Non-numeric entries are ignored.
- A PID directory without `stat` is skipped and not counted.
- `stat` present but `status` missing means the process vanished, and it is
  skipped.
- A `status` that is a **directory** forces a non-`NotFound` error even when
  tests run as root, where chmod proves nothing. The result is `Owner::Unknown`.
- A malformed `stat` is counted in `unreadable`.
- A missing root is `Unreadable { ProcessList }`. A missing `/proc/stat` fails
  the sample.
- Command cache: a D-state process with no cached command shows `[comm]`; a
  changed comm re-reads the cmdline; an unchanged one does not.
- An unknown uid (4000000000) is `Owner::Uid`. Under `cfg(unix)`, uid 0 is
  `Named("root")`.
- Each sample calls `Owners::forget_failed_lookups()` before it resolves
  owners.
- One `cfg(target_os = "linux")` smoke test samples the real `/proc`, finds
  `std::process::id()`, and checks its `StartTime` against `/proc/self/stat`.

### 10.4 Models

- **Units.** `Pid::new(0)` and `Pid::new(i32::MAX + 1)` are `None`.
  `Niceness` bounds and `step` saturation. `Percent::ratio` returns `None` for a
  zero whole and rounds at the boundary. `MemoryBytes::from_kib(u64::MAX)` is
  `None`. `SettableClass` never produces Realtime.
- **Derivation.**
  - The first sample is all `Pending`; the second is `Measured` with exact
    hundredths.
  - A reused PID with a new start is `Pending`, never huge.
  - Counters that go backwards give 0. The cap is `ncpu × FULL`.
  - Meter segments sum to `FULL` or less.
  - The Mem formula matches the worked example in 6.2. Swap and Tasks counts are
    checked.
  - `CpuMeters::grouped` for 1, 2 and 6 cores per bar.
- **View.**
  - Every `SortKey` in both orders, with the PID tie-break.
  - Choosing a key leaves tree view.
  - `FilterTerms`: `a|b`, case, empty terms.
  - `UserScope` and `KernelThreads`.
  - Tree: orphans; a parent that started after its child becomes a root; a
    2-cycle and a self-parent are broken; a 10 000-deep chain without recursion;
    guides at depth 64 and beyond.
- **Selection.**
  - The cursor keeps its key across `apply_sample` and re-sorts. A vanished key
    keeps the clamped index. An empty list has no cursor.
  - The offset follows `visible_window`.
  - `c` marks descendants; `*` inverts; mask marking is reported as
    `Marked { changed, total }`.
  - Search wraps. PID jump: 10 digits at most, never above `i32::MAX`.
- **Header.** The table in 2.3, row for row, plus 0..=60 rows × {0, 1, 4, 8, 32,
  128} cores, where page is at least 1 and chrome + page = rows whenever rows ≥ 7.
- **Schedule.** Every transition in section 7 as a table: not due at
  `t + REFRESH_INTERVAL − 1 ms`, due at `t + REFRESH_INTERVAL`; warm-up after the
  first sample; a held `Automatic` result is discarded; Ctrl+R while held samples
  once.
- **Targets.** Refusal for the own PID, PID 1 against PID 2, a kernel thread,
  `Start::Unknown`. An ancestor gets `Relation::Ancestor`. A cyclic parent chain
  terminates. Frozen targets survive an `apply_sample` that reorders the rows.

### 10.5 Keymap (`action/tasks/tests.rs`)

- One parametrised table covers every binding in 3.1: `(KeyCode, modifiers)` to
  `TaskCommand`.
- It includes `Char('K')`, `Char('k') + SHIFT` and `Char('K') + SHIFT`, all
  `ToggleKernelThreads`, while `Char('k')` is `SignalMenu`.
- `+` with Alt is a search. Alt+j searches for `j`. Ctrl+F4 is `Ignore`.
  Unknown keys are `Ignore`.
- **Help consistency.** Every key named in `HELP` parses and resolves to a
  command other than `Ignore`.

### 10.6 Popup and ops

These go through `process_popup_input` and `popup::tests::key`, with a
`FixtureHost`.

- F8 then Enter records `(4242, Terminate)`; Tab then Enter records `Kill`;
  Esc records nothing.
- F9, Down to SIGHUP, Enter, Enter records `Hangup`.
- Frozen targets are used even after a snapshot moved the rows.
- Marked sets reach every target, in view order.
- The own PID gives a notice and no call.
- F7: an invalid value disables Set; Up and Down stay in −20..=19. `]` sends
  `Nice(n − 1)`, and a second `]` steps from the recorded value.
- Each `ControlError` gives its exact sentence.
- The filter bar edits live, Enter keeps the filter and Esc clears it. Pasted
  text reaches the bar.
- Every `Modal` closes with Esc.
- `ops::tasks::open` opens the viewer with a host installed, and the report with
  none. The Change drive `Process list` row does the same through
  `select_drive`, and the closing drive dialog never takes the viewer down.
- `report_command()` is pinned per platform.

### 10.7 Platform against the real kernel

These tests follow the `platform/tests.rs` pattern and only touch `sleep 30`
children they spawned, while those still hold their PIDs. A reaped child's PID
may already belong to a stranger, so no raw call (`kill`, `setpriority`,
`pidfd_open`) goes to it: the platform wrappers take their `ESRCH` from PID
`i32::MAX`, which is above every kernel's PID ceiling. Each child sits in a
guard that kills and reaps it on drop, so a failed assertion leaves no process.

- `signal` with the right start: the child exits by signal 15.
- A wrong start is `Replaced`, and the child is still alive.
- After the child is reaped, the result is `Gone`, or `Replaced` should its PID
  have been reused meanwhile; the start-time check keeps either one from
  reaching the newcomer.
- `set_niceness` one step above the nice the child inherited reads back from
  `/proc/<pid>/stat`, so a suite run under `nice` still passes.
- Lowering nice one step is `PermissionDenied` unless the kernel's `can_nice`
  rule allows it: `CAP_SYS_NICE` in the initial user namespace, or an
  `RLIMIT_NICE` of at least 20 − the new value. `is_root()` alone is wrong in a
  container without `CAP_SYS_NICE`. Both branches assert.
- The unpinned path is run directly: `signal_process` ends a child.
- The pure `classify_open` is tested with injected `ENOSYS`, `EPERM`, `ESRCH`
  and `EBADF`, so the fallback lines are covered and their mutants killed.
- `clock_ticks_per_second() == 100` under `cfg(target_os = "linux")` (USER_HZ is
  100 on every Linux target Rust supports). This kills the mutants that return 0
  or 1.
- `page_size()` equals the first `KernelPageSize:` of `/proc/self/smaps` × 1024,
  which is portable across arm64 page sizes.

### 10.8 Controller and loop

`application/task_jobs/tests.rs` uses `GatedReader` and synthetic `Instant`s.

- There is no thread and no request while the viewer is closed.
- The first request goes out at once, and a second is never sent while one is
  in flight.
- A session change retires the worker; the new session starts `Pending`.
- `Overdue` at exactly 5 s, not at 5 s − 1 ms. Abandon at exactly 10 s.
- There is no respawn while the abandoned thread lives, and one after it
  finishes.
- `Disconnected` gives `WorkerStopped`.
- `Redraw::Needed` only on a change.
- Dropping `TaskJobs` with a gated read that never returns completes inside
  `test_support::run_with_timeout`.

`application/tests/task_refresh.rs` sits beside `idle_render.rs`. Using
`run_app_with_wsl_backend` with `None` ticks and `CountingBackend`, it checks
exactly one draw per applied sample and zero while closed. The existing
`idle_render` tests pass unchanged.

### 10.9 Screen (`ui/tasks/tests*.rs`)

These draw on `WritableTestBackend` at 80x24, 120x40 and 40x10.

- **Text, checked to the character.** The sketch in 2.1; the formatters at
  every unit boundary (`59:59.99`, `1h00:00`, `99h59:59`, `4d05h`; `99.9`, `100`,
  `3200`); footer and title variants.
- **Styles, checked where the style is the behaviour.**
  - the cursor row's `bg == palette::CURSOR_BG`;
  - `MARKED` on marked rows;
  - the sort title's `CURSOR_BG`;
  - the meter segment colours;
  - `STATE_ALERT` on `D` and `Z`.
- **Layout.**
  - The column drop order at widths 20..=200, with `Cells::of(row) == width`.
  - Control, bidi and CJK names come out as U+FFFD and stay aligned.
  - At every x of the key bar, `task_click_at` agrees with
    `fkey_at(width, 12, x)`, mirroring
    `tests_render_widths.rs:874`.
  - Title clicks agree with the drawn columns, with and without scroll.
  - Menu lines agree with `choices::row_at`.
- **Tracker.** The tracker is cleared in Browse and records an area when the
  confirmation is up.
- **Degenerate sizes.** `tests/degenerate_size_test.rs` adds Tasks empty and
  populated, at every size from 0x0 to 24x24.

### 10.10 TUI BDD (`tests/tui_features/tasks.feature`)

`Session::tasks` installs a `FixtureHost` with four fixture processes, so the
scenarios are identical on `ubuntu-latest` and `windows-latest`. The scenarios:

1. Ctrl+W, then `the screen eventually contains "python3 train.py"`, `"1Help"`
   and `"Task list"`.
2. F6, choose `Memory`, Enter: the largest-RES fixture is the first row.
3. F4, type `nginx`: `Filter: nginx` is shown and `python3` is not.
4. F8, Enter: the fixture host recorded `Terminate` for 4242, and eventually the
   row is gone (the fixture drops a terminated key).
5. `type "K"` shows the kernel-thread fixture `[kthreadd]`.
6. F10 returns to the panels.

`press "K"` sends `Char('k')` with no modifiers, which is Signal menu. Scenarios
therefore use `type "K"` or `press "Shift+K"`.

### 10.11 Existing tests to change (rewrite, never delete)

| Test | Change |
|---|---|
| `ops/menu/tests.rs:320-326` | Splits into a Tasks case (fixture host, `Overlay::Tasks`) and a Devices case |
| `dispatch/tests/far_menu_families/tests_command_dialogs.rs:57` | Expects `Overlay::Tasks`, with a fixture host installed |
| `popup/tasks/tests.rs` | Rewritten for the new popup |
| `popup/drives/tests.rs` (`the_process_list_row_opens_the_task_list_and_the_menu_stays_closed`) | Installs a fixture host, so the row opens `Overlay::Tasks` and never runs `ps`; a second case without a host expects the report |
| `ui/tests_stub_dialogs.rs:38` | Drops Tasks |
| `ui/tests.rs:373` | Uses `render_key_bar` |
| `action/tests_far_menu_keys.rs:101`, `action/tests/fallback_rows.rs:42-45`, `tests/menu_commands_test.rs` | Stay valid unchanged |

### 10.12 Gates

- **`./ffm.py lint`** (and `./ffm.py lint self-test` for the scripts).
  - fmt, for the root crate and `mcp/`.
  - `typos` over every text file.
  - clippy all + pedantic + nursery and the reliability lint set with
    `-D warnings`, over default, `--all-features` and
    `--no-default-features --features test-helpers`, then `mcp/` and
    `colorer-sys/`. `clippy.toml` lets tests unwrap, expect, index and panic;
    a test module counts as test code only under a plain `#[cfg(test)]`.
  - `#[expect(lint, reason = "…")]` only on the narrowest item, and only where
    the code is right as written; `#[allow]` always states its reason.
  - The 30-line, nesting-4 and cognitive-25 limits; no wildcard imports.
  - The architecture tests.
- **Architecture tests.**
  - `module_boundaries` with the new rows.
  - `unsafe_boundary`, unchanged until phase W.
  - `file_line_limit`: 1000 hard, 300 by design.
  - `public_api_coverage`: every new `pub` or `pub(crate)` fn is named in a
    test, and there are no new `UNEXERCISED` entries. Distinctive names such as
    `task_click_at`, `name_of_uid`, `render_key_bar` and `system_host` avoid
    trivial substring passes.
- **Coverage.** At least 98 % of lines under llvm-cov, with **no new
  ignore-regex entries**. Every Linux line is reached by fixture, fake-procfs or
  real-kernel tests, including the ENOSYS path through `classify_open`.
- **Mutation.** `cargo mutants --in-diff` must be clean. Every comparison has a
  boundary test (5 s and 10 s, 1.5 s, the 14-row header threshold, the 100 %
  and 1 h format switches, the Niceness bounds). Thin wrappers are killed by the
  real-kernel tests. No `exclude_re` is added in the first release.
- **`cargo deny`.** No new crate, and `Cargo.lock` unchanged.

In phase W, `cargo mutants --list --in-diff` decides, with evidence, whether the
`cfg(windows)` FFI file needs a justified `exclude_globs` entry.

---

## 11. Documentation to update (in the same branch)

| File | Change |
|---|---|
| `docs/knowledge-base/06-ui-and-interaction/overlays-and-modals.md` | Lines 15, 51 and 53 (Tasks is no longer a stub), the full-screen overlay list, nested modals inside Tasks |
| `docs/knowledge-base/06-ui-and-interaction/rendering-and-layout.md` | The Task list layout; `render_key_bar`; the shared header shape |
| `docs/knowledge-base/06-ui-and-interaction/keyboard-handling.md` | Ctrl+W in the main bindings; Task list modal routing and `normalize` |
| `docs/knowledge-base/09-far-manager-compatibility/hotkeys-and-parity.md` | A Ctrl+W row; a new "Task list (Ctrl+W)" section with 3.1 and the clash table 3.5; a note that Ctrl+F3 and Shift+F3 never arrive on non-kitty terminals |
| `docs/knowledge-base/07-core-features/main-menu.md` | Line 90 row; Verification lines 131-135 (the task list now signals processes, after confirmation) |
| `docs/knowledge-base/02-architecture/system-architecture.md` | Lines 198-222: the Tasks sampler as a second background worker; redraw on a sample; replay is read-only |
| `docs/knowledge-base/02-architecture/module-structure.md` | Rows 49 and 99; the new modules (`process_table`, `application/task_jobs`, `ui/tasks/*`, `models/tasks/*`); the architecture-checks table (new forbidden paths) |
| `docs/knowledge-base/05-testing/testing-strategy.md` | Seams table rows: `ProcessHost`/`FixtureHost`, the fake procfs root, `pump_at(now)`, the BDD eventually step |
| `docs/knowledge-base/03-rust-best-practices/type-system.md` | `Pid`, `StartTime`/`Start`/`ProcessKey`, `Percent`, `MemoryBytes`, `Niceness`, `SettableClass`, `Signal`, `Schedule` |
| `docs/knowledge-base/08-platform-support/windows-behavior.md` | Windows keeps the tasklist report until phase W |
| `README.md` | The hotkey guide (lines 54-84) gains Ctrl+W; the "System tools" bullet (line 24) |
| `src/ui/overlays.rs` | `HELP_LINES` (4.4) |
| `src/action/commands.rs:132`, and the purpose lines of the rewritten tasks files | Doc comments |
| `docs/superpowers/specs/2026-09-13-far-menu-complete-design.md` | C13 (line 376): marked superseded by this file |
| `docs/superpowers/specs/2026-09-22-change-drive-and-netbox-design.md` | Section 3.1, plugin rows: `Process list` opens this viewer, full screen, not a dialog |
| `docs/knowledge-base/09-far-manager-compatibility/hotkeys-and-parity.md` (Alt+F1/F2 row), `docs/knowledge-base/06-ui-and-interaction/overlays-and-modals.md` (DriveSelection) | The `Process list` row opens the live viewer, as Ctrl+W does |

---

## 12. Decisions and alternatives

| # | Decision | Why | Alternatives rejected |
|---|---|---|---|
| D1 | Reuse `Overlay::Tasks(Box<TaskList>)` and the `ops → popup → ui` wiring. Full-screen like the viewer. | One slot already routed from every entry point; AGENTS.md wiring; room for the table | A new overlay variant (a second path to maintain); a centred dialog (too small for htop's table) |
| D2 | Far-native key resolution: Far wins where it has a process meaning, htop fills the gaps, and every htop key that loses opens a harmless dialog | ffm users keep F3/F8/Del/Ctrl+R/`+ - *`/Alt+letter; htop users keep F4/F5/F6/F9/F10 and most letters; no reflex can kill without a second step | Pure htop parity (F8 = nice +, silently irreversible for non-root; F3 = search); htop only |
| D3 | Linux reads `/proc` with `std::fs`, adds the rustix `param` feature and no crate | No unsafe, complete fields (state, PF_KTHREAD, priority, nice), fixture-testable, no dependency cost | `sysinfo` 0.38 (no nice or renice, pulls `windows` 0.62, ntapi and objc2-io-kit, poorer Linux detail); `procfs` 0.17 (Linux only, extra dependencies, foreign error types) |
| D4 | One LoopState-owned worker, request/response, one request in flight, never joined, stall watchdog | A cmdline read can hang; the UI must not; bounded threads and memory; follows `WslJobs` | Sampling in `pump_background` on the UI thread (can block); a free-running timer thread (queues, harder to test); joining on close (can hang quit) |
| D5 | The reader returns raw counters; models derive CPU%, meters and order | One formula for all platforms; pure and testable in `models` | Deriving in the adapter (duplicated per platform) |
| D6 | Kill and renice run synchronously on the UI thread in `ops` | They work while sampling is stuck, when they are needed most; immediate feedback | Routing actions through the sampler (blocked behind a hung read) |
| D7 | Identity = `(Pid, Start)`; pidfd plus start-time re-check | Closes the PID-reuse race on Linux 5.3+; the same key serves cursor, marks, CPU baselines | Kill by PID (can hit a newcomer); a start re-check without a pidfd (a window remains) |
| D8 | Refuse FFM itself, PID 1 and kernel threads before any syscall | STOP to FFM freezes it; SIGINT to systemd reboots; kernel threads ignore signals | htop's allow-all; refusing only FFM |
| D9 | Every signal confirmed, independent of Options > Confirmations | Reliability first; a refresh can move rows; Far's Del is too sudden for a live list | htop's single Enter; following the Delete confirmation setting |
| D10 | The cursor follows its process identity | The highlighted row is always the process the user means (safety); Far keeps the selection after a reload | htop's default of keeping the row index, plus `F` |
| D11 | Kernel threads hidden by default, `K` shows them, the footer counts them | 180 kthreads on this host bury the useful rows; htop 3 hides them by default *(knowledge)* | Shown by default |
| D12 | Mem used uses the htop 3.x segments (6.2) | "Like htop"; segments sum to Total − Free by construction | procps `free`'s Total − MemAvailable (one line to switch; open question 4) |
| D13 | Integer maths; panic-freedom enforced by the manifests' reliability lint set and the robustness corpus; no module-level lint attributes | Release aborts on panic and does not check overflow; `scripts/test_lint_policy.py` keeps the lint policy in the manifests, requires each family at `warn` (the gate's `-D warnings` makes it an error), and no module in `src` sets its own lint levels | `#![deny(clippy::unwrap_used, arithmetic_side_effects, …)]` per module |
| D14 | The host lives in `App.task_host`, installed by bootstrap; replay wraps it read-only | Keeps `run_loop` and `run_app_with_wsl_backend` signatures unchanged; BDD and integration tests install a fixture as they install a clipboard | A `Backends` struct threaded through `run_loop` (changes public test-helper API) |
| D15 | Platforms without a native reader keep today's report | No regression on Windows and macOS at any merge | Showing "not available" (regresses today's report) |
| D16 | The command line is read once per identity and comm; not read for uncached D-state processes | htop's default; cuts cost and the mmap-lock hazard; `execve` changes comm, so a re-read follows | Re-reading every refresh; reading once ever (misses `execve`) |
| D17 | Menus reuse the generic `menu_dialog::choices`; the mask dialog, `MaskPrompt` and `Mask` are reused for Gray `+`/`-` | Real reuse, no churn to the panel Sort menu | Extracting a new check-menu from `ui/sortmenu.rs` (more churn for the same result) |
| D18 | Far-style `abbreviate_bytes` for sizes | Consistent with the panels, the drives dialog and the delete dialog | htop's `381M` (a second format) |
| D19 | `Redraw` moves to `application.rs` | Two owners now (`WslJobs`, `TaskJobs`); it is a loop concept, not a WSL one | Importing `wsl_jobs::Redraw` from `task_jobs` |
| D20 | Linux first; Windows (W) and macOS (M) as later phases behind the same ports | The user asked for Linux; macOS has no CI; Windows needs reviewed unsafe | Shipping all three at once |
| D21 | Session-only settings in the first release | KISS; persistence needs a settings page and a save rule | Saving automatically on close (open question 1) |

---

## 13. Open questions for the user

1. **Settings.** Should the sort, the refresh interval and the toggles (kernel
   threads, tree, command view) be saved in `settings.yaml` with Shift+F9 Save
   setup, as Far does, or stay session-only, as proposed for the first release?
2. **PID 1.** Signals to PID 1 are refused, including SIGHUP, which root users
   sometimes send to reload init. Keep the blanket refusal, or allow SIGHUP only?
3. **Confirmation setting.** Every signal is confirmed, independent of
   Options > Confirmations. Should a user be able to turn that off, as with
   Delete?
4. **Mem "used".** Should it use htop's segments, as proposed, or procps `free`'s
   `MemTotal − MemAvailable`? On this host they are 38.2 G against 40.0 G.
5. **Other platforms.** After Linux ships, is the native Windows reader (phase W)
   wanted? Is a `ps`-based macOS reader (phase M) wanted, given that macOS is not
   in CI and must be checked by hand?
6. **Renice without confirmation.** `]` and `[` renice at once, as in htop.
   `[` (nice +1) cannot be undone by a non-root user. Keep that, or confirm when
   not root?

---

## 14. Implementation tasks (first release, in order)

Each task leaves `./ffm.py lint` and
`cargo test --features test-helpers` green.

| ID | Task | Files | Depends on | Tests |
|---|---|---|---|---|
| T01 | Move `visible_window` to `crate::scroll` | `src/scroll.rs`, `src/scroll/tests.rs`, `src/ui/panel.rs`, `src/ui/panel/modes.rs`, `src/ui/tree.rs`, `src/ui/panel/tests_window.rs` | — | Existing window tests moved unchanged; panel and tree render tests stay green |
| T02 | Generalise the key bar | `src/ui/fkey_bar.rs`, `src/ui.rs`, `src/ui/tests.rs`, `src/ui/tests_render_widths.rs` | — | Panel bar byte-identical; the agreement test at `tests_render_widths.rs:874` passes for a 12-label custom set too |
| T03 | Generic choices menu and shared `navigate` | `src/ui/menu_dialog/choices.rs`, `src/ui/menu_dialog.rs`, `src/models/menu_dialog.rs`, `src/popup/menu_dialog.rs` | — | Existing `ui/menu_dialog/tests.rs` unchanged; a new test renders a `&[&str]` menu and hit-tests it |
| T04 | Reuse hooks | `src/ui/panel.rs` (visibility), `src/ui/dialog.rs` (`preview_lines`), `src/ui/overlays.rs` (`render_mask_prompt`, `DELETE_PREVIEW`), `src/popup/mask_prompt.rs` (`mask_key`) | — | Delete dialog text unchanged; `preview_lines` for 0, 5 and 6 names; mask prompt tests unchanged plus `mask_key` outcomes |
| T05 | `Owners::name_of_uid` | `src/directory/owner.rs`, its tests | — | uid 0 is root on Unix; unknown uid is `None`; one lookup per uid (counted); `of()` unchanged |
| T06 | Move `Redraw`; add layer rules | `src/application.rs`, `src/application/wsl_jobs.rs`, `tests/module_boundaries/layers.rs` | — | Existing loop tests; module-boundary tests with the new rows |
| T07 | Model value types | `src/models/tasks/{identity,units,priority,signal}.rs`, tests, `src/models.rs` | — | 10.4 units |
| T08 | Raw sample, ports and errors | `src/models/tasks/sample.rs`, tests | T07 | `ProcessState::from_letter` table; error `Display` sentences |
| T09 | Platform wrappers and rustix `param` | `Cargo.toml`, `src/platform.rs`, `src/platform/unix.rs`, `src/platform/tests.rs` | T07 | 10.7 real-kernel tests |
| T10 | Snapshot and meters derivation | `src/models/tasks/{snapshot,meters}.rs`, tests, `tests_support.rs` | T07, T08 | 10.4 derivation |
| T11 | View: sort, filter, scope, tree | `src/models/tasks/{view,tree}.rs`, tests | T10 | 10.4 view |
| T12 | `TaskList` state | `src/models/tasks.rs`, `src/models/tasks/{selection,schedule,header,modal,targets,host}.rs`, tests, `src/models/app.rs`, `src/models/app_overlay.rs`, `src/models/overlay.rs` | T01, T10, T11 | 10.4 selection, header, schedule, targets; `set_viewport` clamps Tasks |
| T13 | procfs parsers and robustness corpus | `src/process_table/procfs/parse.rs`, `tests_parse.rs`, `tests_robustness.rs` | T08 | 10.1, 10.2 |
| T14 | `ProcFs` reader, command cache, `SystemHost`, `system_host()` | `src/lib.rs`, `src/process_table.rs`, `src/process_table/procfs.rs`, `tests_reader.rs` | T05, T09, T13 | 10.3 |
| T15 | Linux identity-checked control | `src/process_table/procfs/control.rs`, `tests_control.rs` | T09, T13, T14 | 10.7, including the injected `classify_open` errnos |
| T16 | Test fixtures | `src/test_support/processes.rs`, `src/test_support.rs` | T08, T12 | Self-tests: canned samples advance, controls are recorded, a terminated key disappears, `GatedReader` blocks until opened |
| T17 | Controller and loop wiring | `src/application/task_jobs.rs`, `tests.rs`, `src/application.rs`, `src/application/timing.rs`, `src/application/tests/task_refresh.rs` | T06, T12, T16 | 10.8 |
| T18 | Keymap and help table | `src/action.rs`, `src/action/tasks.rs`, `src/action/tasks/tests.rs` | T07, T11 | 10.5, including help consistency |
| T19 | Ops, rerouting, bootstrap and replay | `src/ops/tasks.rs`, `src/ops/tasks/{report,control}.rs`, tests, `src/ops/menu.rs`, `src/ops/menu/workflows.rs`, `src/popup/drives.rs`, `src/bootstrap.rs`, `src/recording/processes.rs`, `src/recording.rs`, `src/action/commands.rs` | T12, T14, T15, T16 | 10.6 ops; the replay host returns `ReadOnly`; `report_command` per platform; rewritten `ops/menu/tests.rs:320`, `tests_command_dialogs.rs:57` and the drive row test in `popup/drives/tests.rs` |
| T20 | Popup: router, browse, bars, dialogs, menus, paste | `src/popup.rs`, `src/popup/tasks.rs`, `src/popup/tasks/{browse,bars,dialogs,menus}.rs`, tests, `src/popup/editor.rs`, `src/application.rs` (paste) | T03, T04, T18, T19 | 10.6 popup; rewritten `popup/tasks/tests.rs` |
| T21 | Renderer | `src/ui/tasks.rs`, `src/ui/tasks/{meters,table,format,chrome,dialogs}.rs`, tests, `src/ui/palette.rs`, `src/ui/overlays.rs` (arm, `HELP_LINES`), `src/ui.rs`, `src/ui/tests_stub_dialogs.rs`, `tests/degenerate_size_test.rs` | T01, T02, T03, T04, T12 | 10.9 |
| T22 | Mouse | `src/application/dispatch/dialog_mouse.rs`, `src/application/dispatch/dialog_mouse/{tasks,button_keys}.rs`, tests | T20, T21 | Row, title, key-bar and menu-line clicks become their keys or transitions; confirmation buttons map to `t`/`k`/Esc; a click never signals |
| T23 | BDD | `tests/tui_bdd/{session,mod,tasks}.rs`, `tests/tui_features/tasks.feature`, `tests/far_manager_hotkeys_test.rs` | T16, T20, T21, T22 | 10.10 on ubuntu and windows; the Ctrl+W hotkey case |
| T24 | Benchmark (measure first) | `benches/tasks_bench.rs`, `Cargo.toml` | T11, T14 | Numbers recorded in the PR description; no optimisation without a before and after |
| T25 | Documentation | All files in section 11 | T19, T21 | `./ffm.py lint` (docs and spelling checks); `tests/private_data_lint_test.rs` |
| T26 | Final gates | — | T01-T25 | Coverage ≥ 98 % with no new ignores; `cargo mutants --in-diff` clean; `cargo deny`; Windows CI green (report fallback plus viewer via fixtures) |

**Later phases** (separate branches, each squash-merged; not in the first
release):

| Phase | Deliverable |
|---|---|
| **W** | Windows reader: `src/platform/windows/processes.rs` (a new `BOUNDARIES` entry) and `src/process_table/windows.rs`, with pure conversions under `cfg(any(windows, test))`. Pinned `TerminateProcess` with the `IsProcessCritical` check; `SetPriorityClass` with `SettableClass`; F9 behaves as F8; `system_host()` becomes `Some`; the tasklist branch is removed; `windows-behavior.md`; the mutants decision recorded with evidence. |
| **M** | `src/process_table/ps.rs` for macOS and BSD. A pure parser with five-token `lstart`, tested on Linux; the runner is `ops::menu::process::capture`; verify-then-act kill and renice; checked by hand on a Mac. When every supported platform has a reader, `ops/tasks/report.rs` is deleted. |

---

## 15. Risks

| Risk | Mitigation |
|---|---|
| A cmdline read hangs on a process with a stuck mmap lock | Worker thread, no D-state reads while uncached, overdue and stall states, kills still work. One abandoned thread may remain. |
| The Windows idle loop may spin (`tty::wait` returns zero, inferred from code) | Per-pass cost is a `match` plus `try_recv`. The loop itself is not fixed here. |
| A check-then-act window without a pidfd (old kernels, seccomp), for renice, and on macOS | Documented. The worst case for renice is one nice step on a newcomer. |
| Header height depends on the CPU count and the terminal height | One `HeaderShape` in models, with agreement tests on page, layout and clicks. |
| BDD has no idle ticks | The FocusGained eventually step, bounded by the 10 s `DEADLINE`; assertions on content, never on timing. |
| htop facts from knowledge (1.5 s default, Mem formula, kernel threads hidden by default) | Each is one constant or function, pinned by tests, and listed as a decision or an open question. |
| cargo-mutants runs only on Linux, and `cfg(windows)` code is not compiled there | Keep FFI thin; logic in `cfg(any(windows, test))` pure functions; decide exclusions in phase W with evidence. |
| `overlays.rs` is at 840 lines and `panel.rs` at 888 (after T01 and T04) | The design only moves one arm and raises visibility. `visible_window` leaves `panel.rs`. All new code is in new files. |
| Linux `setpriority` renices only the thread-group leader | Stated on the help page and in the parity doc, as `renice(1)` behaves. |
| htop users expect F3, F7 and F8 to search and renice | Each lands on a harmless dialog. The help page and the parity doc list the differences. |

---

## Appendix A. Corrections to earlier proposals (verified)

1. **The brief.** It said Ctrl+W opened the Tasks stub. It does not; see 1.2.
2. **`<` and `>`.** In htop they open the sort selector
   (man page: "F6, <, > Selects a field for sorting"). They do not step to the
   previous or next key.
3. **Gray `+` and `-`.** In ffm they open the Select/Deselect mask dialog
   (`dispatch/actions.rs:136-137`); they do not mark all or unmark all. This
   design keeps that meaning.
4. **Shift+F3 and Ctrl+F3.** On terminals without the kitty protocol crossterm
   0.29 reads them as a cursor-position report (`hotkeys-and-parity.md:146`).
   Shift+F3 is not bound, and no bar advertises it.
5. **`Redraw`.** Only `WslJobs` returns it today; `EditorLspController::pump` and
   `ops::vault::maintenance` return `bool`. It moves because it gains a second
   owner.
6. **BDD `press`.** A single character is lowercased with no modifiers. SHIFT
   comes only from `Shift+`. A bare `F` panics in the F-key arm. Section 4.4 fixes
   the harness; section 10.10 uses `type` for uppercase letters.
7. **Lint policy.** It lives in the manifests (`scripts/test_lint_policy.py`,
   which requires the reliability families at `warn` in every manifest), and
   no `src` module sets its own lint levels. Hence D13.
8. **Release profile.** It is `panic = "abort"` and `overflow-checks = false`
   (`Cargo.toml:294-302`).
9. **Key-bar agreement test.** It is at `ui/tests_render_widths.rs:874`, not 872.
10. **Mem formula.** An earlier proposal counted Shmem twice: once inside "used"
    and again as the shared segment. The formula in 6.2 sums to Total − Free.
11. **macOS `lstart`.** It contains spaces. It is parsed as exactly five tokens
    (6.4).
12. **`clock_ticks_per_second() >= 1`.** That assertion does not kill a mutant
    returning 1. The test asserts `== 100` on Linux.
13. **`Debug` on `TaskList`.** It would expose argv. `TaskList` has no `Debug`.
14. **The moved report.** Moving it verbatim would keep `if cfg!()` inside one
    function. It is split into same-signature cfg functions.
15. **The pidfd fallback.** It had no test seam. `classify_open` is pure and
    tested with injected errnos.
16. **New finding: replay.** Replaying a recording that contains a kill would
    signal a real process. The replay host is read-only (S8).
17. **rustix 1.1.5 facts.**
    - `param = []`.
    - `pidfd_open` and `pidfd_send_signal` are `cfg(target_os = "linux")`.
    - `Signal` is re-exported as `rustix::process::Signal`.
    - `setpriority_process(None, …)` means the calling process.
    - `Pid::from_raw` only debug-asserts non-negative input.
18. **windows-sys 0.61.2 facts.**
    - `Win32_System_Diagnostics_ToolHelp` is at manifest line 200 and
      `Win32_System_ProcessStatus` at line 232.
    - `TerminateProcess`, `IsProcessCritical`, `GetProcessTimes`,
      `GetSystemTimes`, `Get/SetPriorityClass`, `QueryFullProcessImageNameW`,
      `OpenProcessToken` and `GetActiveProcessorCount` are under
      `Win32_System_Threading`.
    - `REALTIME_PRIORITY_CLASS` exists, so a reported class needs a `Realtime`
      variant and a settable class must not have one.

## Resolved open questions (first release)

Chosen for reliability; each can be revisited after the first release.

1. Sort, refresh interval and view toggles are session-only; nothing is written to `settings.yaml`.
2. Every signal to PID 1 is refused, SIGHUP included.
3. Every signal is confirmed; Options › Confirmations does not turn it off.
4. The Mem meter's "used" value follows htop's segments.
5. Linux only. Phase W (native Windows reader) and phase M (macOS/BSD `ps` reader) stay deferred; those platforms keep today's `ps`/`tasklist` report.
6. `]` renices at once. `[` (nice +1) asks for confirmation when the user is not root, because a non-root user cannot undo it; as root it renices at once.

## 16. Rebase onto main `0125c57` (2026-09-24)

The first branch, `feat/process-viewer`, was built on `415db60`, a history that
shares no commit with main. Its 30 commits (the spec, T01-T13 and the M1/M2
fixes) were replayed onto main `0125c57` as `feat/process-viewer-2`. What main
changed, and where this file follows it:

| Main's change | Effect on this design |
|---|---|
| NetBox (`9f0370c`): the Change drive dialog's `Process list` row, `DriveEntry::ProcessList`, calling `ops::tasks::open` | The row is an entry point of this viewer, not a deferred item: 1.2, 1.3, 1.4, 4.1, 4.4, 10.6, 10.11, 11 and T19 |
| NetBox: `ui/remote.rs` and `ui/sessions.rs` import `visible_window` | T01 points them at `crate::scroll` too |
| NetBox: `models::RowCursor` and `RowMotion` | The table cursor moves with them (4.5) instead of its own clamp |
| Settings in `settings.yaml` (`e2a4230`) | Open question 1, 1.5 and the resolved answer name `settings.yaml`; nothing is saved in the first release either way |
| Python scripts (`e91ced6`): `./ffm.py` replaces `./ffm.sh`, `scripts/lint.py` | 10.12 and 14 |
| `typos` in the lint gate (`cb4dfcc`) | 10.12; `typos.toml` accepts htop's `thr` |
| The reliability lint set (`0ab5890`): no `unwrap`, `expect`, `panic!`, indexing, string slicing, `as` casts or printing in production code; `redundant_imports`, `unused_qualifications`, `elided_lifetimes_in_paths`, `let_underscore_drop`, `map_err_ignore`, `allow_attributes_without_reason`, `mod_module_files` | 5, 9, 10.12, D13 and A.7. The replayed code was brought up to it: the task list models reach rows through `get()` and a visited set, and the new files keep the `<name>.rs` layout (`application.rs`, `test_support.rs`) |
| `ops::menu::process::capture` kills the whole process tree on timeout (`0125c57`) | The phase M runner inherits it (6.4); the report fallback is unchanged |

## 17. Milestone M4 as built (T14-T17, 2026-09-24)

The reader, the identity-checked control, the fixtures and the controller
landed. Where they differ from sections 4-10 as first written, those
sections now say what was built; in short:

| Where | As built | Why |
|---|---|---|
| 6.1 reader | `/proc` is listed before the system files are read; an entry the listing fails on fails the sample (`Unreadable { ProcessList }`); rows are de-duplicated by sorting on `ProcessKey` | A missing root reports the process list (10.3); a list cut short would look complete |
| 4.3 files | `procfs/files.rs` (caps, buffer), `task_jobs/{worker,logged}.rs`, `test_support/processes/gate.rs`, `test_support/sleeper.rs` | Every file stays under 300 lines; the sleep child is shared by the platform and control tests |
| 5.5, 6.1.3, 9 | `ControlError::Refused(ControlRefusal)`; the host refuses FFM and init before any system call and kernel threads after the re-read; `ControlRefusal::signal_to_pid` and `signal_to_kind` are the one rule both layers use | No caller of `ProcessHost::signal` can reach them, whatever it checked first |
| 6.1.3 | `ProcessControl { root, own, ticks }` holds what signal and renice share; `verify_start` reads another read error through `control_error` and an unparsable line as `Unverifiable` | Fewer parameters; every re-read outcome has a variant |
| 7 | `TaskJobs { sampler: Sampler, logged }`, `Sampler { Idle, Running, Abandoned }`, `Worker.flight`; a reader that cannot open shows its own error; a spawn failure loses the worker for the session; a list reopened behind a stuck read becomes `Failed(WorkerStalled)` at the stall limit; every sample error and stall is logged once per session and kind | Illegal states unrepresentable; honest errors; no list that looks as if it were starting for ever |
| 7 table | A held list's stalled `Automatic` request goes back to `Held` | Its answer would have been discarded; asking again would change a held screen |
| 10.x fixtures | `FixtureHost::new(samples)`, `FixtureHost::gated() -> (FixtureHost, Gate)`, `Control`, `fail(key, error)`, `readers_opened()`; the sample builders live in `test_support::processes` and the models' `tests_support` re-exports them; `Niceness::NORMAL` | One fixture set for tests inside and outside the crate |

Until bootstrap installs the host (T19), `system_host()` carries the one
`dead_code` expectation of the process table; T19 removes it.

## 18. Milestone M5 as built (T18-T20, 2026-09-24)

The keymap, the effects and the popup landed. Where they differ from
sections 2-10 as first written, those sections now say what was built; in
short:

| Where | As built | Why |
|---|---|---|
| 3.1, 8 S4, resolved question 6 | `[` asks for a confirmation (`Modal::Renice(NiceConfirm)`, buttons Renice and Cancel) unless FFM runs as root; the list knows it through `TaskList::new(session, own, privilege)`, with `crate::prompt::Privilege`, the type the shell prompt's `#` already uses | The resolved question; one privilege type, not two |
| 4.3, 5.7 | `models/tasks/renice.rs`: `NicePlan`, `PlannedChange`, `Renice`; `Priority::stepped` refuses a bound, a Realtime class, a class limit (`ClassAtLimit`) and an unread priority (`UnknownPriority`) | Planning is pure and tested; `ops` only applies it |
| 5.7, 9 | `Notice::Stepped { step, count }` for `]` and `[` ("Priority raised one step for 2 processes."); `Failure { target, attempt, error }` words each line, and a failed renice says "nothing was changed" and "priorities are not changed" on replay | Stepped targets may end at different values; a renice is not a signal |
| 5.5 | `ProcessHost: Send + Sync` | The replay host wraps another host behind `Arc` |
| 3.3 | Confirmation letters press their button only without Ctrl | Ctrl+K is a command chord and must not send SIGKILL |
| 3.2 | Backspace in the PID jump moves the cursor to the shorter PID; the filter re-filters only when its text changed; a digit and a list move in a bar are what the keymap resolves them to | One rule for each key |
| 3.3, 2.8 | Enter or F3 on an empty list gives the notice "No process is selected."; Enter on the Priority form's Cancel cancels, and F10 cancels the form too; the help page scrolls by the table's page | The nearest Far behaviour |
| 4.3 | `test_support/task_list.rs` (`TaskApp`: an app with a fixture host and a sampled list); `models/tasks/table.rs`; `crate::scroll::WIDE_STEP`, the one 20-cell pan the viewer, the quick view and the Command column share | Shared fixtures; files within 300 lines; Rule of Three |
| 4.4 | `App::task_list_mut`; `ops::tasks::control::{confirm_terminate, open_signal_menu, press, open_priority, set_nice, step_nice, answer_renice}`; the Unix-only tests (a shell for the report, real signal names) live in files compiled on Unix only | The popup and the mouse (T22) share one effect per button; the Windows cross-lint stays clean |

Until the renderer (T21) lands, `Ctrl+W` opens the live list behind the old
"Coming in this release" placeholder and its keys act unseen: the branch must
not be run against real processes before M6.

