<!-- Purpose: Explains the Overlay state machine and modal behavior. -->

# 🪟 Overlays and Modals

One of the most important architectural features of FunnyFileManager is the centralized handling of modal overlays.

## 🧩 The Overlay Enum

Instead of many independent `bool` and `Option<T>` fields (`show_help`, `viewer`, `command_line`, `show_drives`, `delete_confirm`, `new_file_input`, and so on), the project uses one enum, `Overlay`, defined in `src/models/overlay.rs` and stored in `App.overlay`.

`ViewMode` (`Panels` or `Console`) is a separate field. `Ctrl+O` hides the panels without opening an overlay, so a dialog can sit over either background.

`CommandLine` owns both the editable text buffer and the fuzzy-completion menu state. Keeping them inside the overlay makes menu visibility and selection part of the same mutually exclusive modal state.

The variant list changes as dialogs are added. Read `src/models/overlay.rs` rather than a copy of it. The groups today are help, the viewer, and the editor; drive selection; file prompts (new file, delete, quit, rename and mkdir, copy, wipe, archive, attributes, apply command, describe); the F9 menu bar and `MenuDialog`; search, history, filter, screens, tasks, and settings dialogs; WSL, the PATH editor, and disk cleanup; the information panel; and, with the `keepass` feature, the vault.

## 👍 Benefits

- Mutual exclusivity is enforced by the type system (only one overlay can be active).
- Pattern matching on the overlay is exhaustive and clear.
- Opening/closing logic is centralized via `app.set_overlay(...)` and `app.close_overlay()`.

## 🪟 Current Overlay Types

- **Viewer** (F3) — full screen, as Far's is: row 0 is a teal status line
  (full path │ `t` │ encoding │ size │ `Col N` │ percent │ clock) and the
  syntax-highlighted file fills every row below. Also reused for MCP status
  and Windows reports, which show only their title and the clock on the bar
  and no highlighting.
- **Editor** (F4) — built-in text editor.
- **Help** (F1)
- **CommandLine** (`:`) — modal buffer plus fuzzy command/path dropdown
- **DriveSelection** (Alt+F1 / Alt+F2) — a drive remembers its current
  directory, as in Far: picking the drive a panel is already on opens that
  panel's folder (the active panel's first), not the drive's root. Under the
  rule: the S3 profiles, `NetBox` (the stored-sessions panel on that side) and
  `Process list` (the Task list); the dialog closes before a row acts, so a
  row that opens a window of its own keeps it.
- **DeleteConfirm** (F8)
- **NewFileInput** (Shift+F4)
- **NamePrompt** (Shift+F6 rename, F7 mkdir, Alt+F6 link)
- **QuitConfirm** (F10)
- **Information** (Ctrl+L)
- **Menu** and **MenuDialog** (F9, and the commands that prompt or list choices)
- **CopyPrompt**, **WipeConfirm**, archive dialogs, **Attributes**, **ApplyCommand**, **Describe**
- **FindFile** (Alt+F7)
- **SortModes** (Ctrl+F12)
- **PluginCommands** (F11) — Far's Plugin commands list: MCP status, NetBox's
  stored sessions, the NetBox `.netbox` import
- **SessionEdit** (F4 / Shift+F4 on the sessions panel) — one stored session's
  fields; no password field
- **InputPrompt** and **Question** — the one-line input and the yes/no the
  NetBox panels ask with; each carries its purpose (`InputPurpose`,
  `QuestionPurpose`) and `ops::prompts` is the one match from a purpose to
  its operation
- **Kdbx** when the `keepass` feature is on

`Ctrl+O` is `ViewMode::Console`, not an overlay.

## The hotkey and the F9 row

Alt+F8, Alt+F9, Alt+F10, Alt+F11, Alt+F12, Ctrl+I, F12, and Ctrl+W call the same `ops/menu` operation as the F9 row of that name. That is a `MenuDialog` prompt or choice list, the task-list viewer, or `ops::videomode::toggle`, which asks the terminal to resize and reports the height it got in the status bar. Alt+F7 opens the Find file dialog; its containing-text field is part of the search.

The older dedicated variants (`CommandHistory`, `FindFolder`, `ViewHistory`, `FolderHistory`, `Filter`, `Screens`, `Tasks`) still exist and still draw "Coming in this release" if something sets them. The hotkeys no longer do.

## Implementation locations

- State: `src/models/overlay.rs`. Accessors such as `get_viewer` live in `src/models/app_overlay.rs`.
- Input: `src/popup.rs` → `process_popup_input` (re-exported via `src/handlers/mod.rs` for compatibility).
- Effects: `src/ops/`.
- Opening from the main view: `src/application/dispatch.rs`. `src/lib.rs` re-exports `run_app`; it does not route keys.
- Rendering: `src/ui/` children, routed from `src/ui/overlays.rs`. The viewer delegates to `src/ui/viewer.rs`. Highlighted text shared with the editor lives in `src/ui/code.rs`.

Every variant must be reachable from a key or a menu row and have an intentional presentation. A variant that is only constructed in tests is dead weight: it makes `process_popup_input` swallow every key while nothing is painted. There was once a `Search(String)` and a `Bookmarks` variant in exactly that state; both were deleted. Incremental search is `Alt+letter` (quick search); bookmarks are `:bookmark set|go <0-9>`.

While an overlay is active, every key is routed to `process_popup_input` before
main-view action resolution. Unsupported keys remain consumed; they cannot fall
through to panel operations.

## 🧑‍💻 Compatibility Accessors

Helpers such as `get_viewer`, `get_command_line`, and
`get_delete_confirm` project data from the active enum variant. They do not
create parallel modal fields; new modal state still belongs in `Overlay`.
