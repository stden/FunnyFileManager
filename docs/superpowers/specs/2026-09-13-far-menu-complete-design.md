<!-- Purpose: Design for making every row of the F9 menu do what Far Manager's does. -->

# Far Manager menu, complete: design

_2026-09-13. Status: open plan for rows that are still unfinished. The hotkeys Alt+F8, Alt+F9, Alt+F10, Alt+F11, Alt+F12, Ctrl+I, F12, and Ctrl+W now call the same `ops/menu` operation as their F9 rows. This file remains the plan, not a description of every current dialog. See `docs/knowledge-base/06-ui-and-interaction/overlays-and-modals.md`._

## 1. Goal

Every row of the F9 menu bar — Left, Files, Commands, Options, Right — does
what the same row does in Far Manager 3, adapted to a terminal file manager
that runs on Linux, macOS and Windows. No row is greyed. The cursor therefore
moves one row at a time, which is the fix for "the arrows jump strangely":
today the cursor skips every greyed row, and in a drop-down where most rows
are greyed that looks like a leap across groups.

The authoritative source for each row's meaning is Far's own help,
`far/FarEng.hlf.m4` in `FarGroup/FarManager` (a copy is kept under the
session scratchpad while this work runs; the topic index is the `@Name`
lines). Menu topics: `@LeftRightMenu`, `@FilesMenu`, `@CmdMenu`, `@OptMenu`.

## 2. Principles carried over

- **The menu is a second route, not a second implementation.** A row with a
  key runs that key through the normal dispatcher (`Run::Key`). A row Far
  gives no key — Compare folders, Edit user menu, File associations, Folder
  shortcuts, Hotplug devices, every Options page — carries a typed
  `MenuCommand` that becomes `Action::Menu(command)` and goes through the same
  `apply_action` as everything else (`Run::Command`).
- **Enums over booleans; illegal states unrepresentable** (knowledge base
  `03-rust-best-practices/type-system.md`). New modal state is a variant of
  `Overlay`; new per-panel state is a field of `Panel` with an enum type.
- **Presentation reads state.** `src/ui/**` and `src/action/**` never touch
  `std::fs`, `std::process`, `crate::ops`, `crate::directory`, `crate::popup`
  (enforced by `tests/module_boundaries_test.rs`). Anything that must read the
  disk to open a dialog does so in `src/ops/<feature>.rs`.
- **Readability gate**: functions ≤ 30 lines, cognitive complexity ≤ 25,
  nesting ≤ 4 (`clippy.toml`), `-D warnings` across the feature matrix
  (`scripts/lint.sh`), no `src/` file over 1000 lines, unit tests in
  `tests*.rs` child modules only.
- **Honesty over pretence.** Where a Far feature has no meaning on this
  platform (a Windows-only device list on macOS), the row still runs and says
  so in the status bar. It never silently does nothing.

## 3. Shared scaffold (Wave 0 — one agent, sequential, main tree)

The scaffold lands first so that every feature can then be built in an
isolated worktree touching only its own files. After the scaffold, the program
compiles, every test passes, and every menu row is live — most of them
opening a stub dialog that says what it will do.

### 3.1 `src/menu.rs`

```rust
pub enum Entry { Item { label, shortcut, run: Run, check: Check }, Separator }
pub enum Run { Key(Shortcut), Command(MenuCommand) }
pub enum Check { Never, View(PanelView), Sort(SortMode), Toggle(PanelToggle) }
pub enum PanelToggle { Tree, QuickView, LongNames, PanelOn }
pub enum MenuCommand {
    CompareFolders, EditUserMenu, FileAssociations, FolderShortcuts,
    HotplugDevices, Options(OptionsPage),
}
pub enum OptionsPage {
    System, Panel, Tree, Interface, Languages, PluginsConfiguration,
    PluginsManager, Dialog, Menu, CommandLine, AutoComplete, InfoPanel,
    MaskGroups, Confirmations, PanelModes, FileDescriptions,
    FolderDescriptions, Viewer, Editor, CodePages, Colors, Highlighting,
}
```

- `Entry::planned` is deleted; `is_selectable` is true for every `Item`.
- `Menu::is_checked(&self, app: &App, entry: &Entry) -> bool` answers
  `Check` against the panel the heading names (`Heading::side`), so the view
  mode in use carries Far's tick and the toggles tick while on.
- Row tables keep Far's order and letters. The Left/Right tables gain nothing;
  Files keeps the two FFM extras; Commands keeps Go to root and the Windows
  WSL row. The count test moves from "rows present" to "rows present and
  every one live".
- `App.pending_key: Option<Shortcut>` becomes `App.pending: Option<Run>`;
  `dispatch_key` dispatches `Run::Key` as a key event and `Run::Command` as
  `Action::Menu`.

### 3.2 `src/action.rs`

New variants, each with its key in `resolve` (modifier arms before bare arms):

| Action | Key | Far row |
|---|---|---|
| `Panel(PanelCommand::View(PanelView::*))` | Ctrl+1 … Ctrl+0 | Brief … Alternative full |
| `Panel(PanelCommand::Tree)` | Ctrl+T | Tree panel |
| `Panel(PanelCommand::QuickView)` | Ctrl+Q | Quick view |
| `Panel(PanelCommand::SortMenu)` | Ctrl+F12 | Sort modes |
| `Panel(PanelCommand::Sort(SortMode::*))` | Ctrl+F3 … Ctrl+F11 | (keys only) |
| `Panel(PanelCommand::LongNames)` | Ctrl+N | Show long names |
| `Panel(PanelCommand::TogglePanel(side))` | Ctrl+F1 / Ctrl+F2 | Panel On/Off |
| `File(FileCommand::Link)` | Alt+F6 | Link |
| `File(FileCommand::Wipe)` | Alt+Del | Wipe |
| `File(FileCommand::AddToArchive)` | Shift+F1 | Add to archive |
| `File(FileCommand::Extract)` | Shift+F2 | Extract files |
| `File(FileCommand::ArchiveCommands)` | Shift+F3 | Archive commands |
| `File(FileCommand::Attributes)` | Ctrl+A | File attributes |
| `File(FileCommand::ApplyCommand)` | Ctrl+G | Apply command |
| `File(FileCommand::Describe)` | Ctrl+Z | Describe files |
| `Tool(ToolCommand::FindFile)` | Alt+F7 | Find file |
| `Tool(ToolCommand::History)` | Alt+F8 | History |
| `Tool(ToolCommand::VideoMode)` | Alt+F9 | Video mode |
| `Tool(ToolCommand::FindFolder)` | Alt+F10 | Find folder |
| `Tool(ToolCommand::ViewHistory)` | Alt+F11 | File view history |
| `Tool(ToolCommand::FolderHistory)` | Alt+F12 | Folders history |
| `Tool(ToolCommand::Filter)` | Ctrl+I | File panel filter |
| `Tool(ToolCommand::Screens)` | F12 | Screens list (replaces the console toggle on F12; Ctrl+O keeps it) |
| `Tool(ToolCommand::Tasks)` | Ctrl+W | Task list |
| `Tool(ToolCommand::SaveSetup)` | Shift+F9 | Save setup |
| `Menu(MenuCommand)` | — | keyless rows; `resolve` never returns it |

Terminal caveats, documented in the parity doc, not worked around: Ctrl+I is
Tab and Ctrl+M is Enter on the wire, Ctrl+digit is not distinguishable in most
terminals. The menu route always works because the menu builds the key event
itself. `crate::diagnostics::action_label` keeps working through `Debug`.

`UserMenu` (F2) opens the user menu (§5 C8) instead of the Commands heading.

### 3.3 Models

- `src/models/panel.rs`: `Panel` gains `view: PanelView`, `kind: PanelKind`,
  `long_names: bool` (default true), `filter: PanelFilter`,
  `sort_reversed: bool`. `PanelKind { Files, Tree(Box<FolderTree>), QuickView }`.
- `src/models/panelview.rs` (new): `PanelView` (ten Far modes, `Full` default),
  `is_fullscreen()` for Detailed and LongDescriptions, `label()`.
- `SortMode` gains `ModificationTime, Unsorted, CreationTime, AccessTime,
  Descriptions, Owner`; `sort_entries` orders by each (unsorted = listing
  order); the panel's header letter reflects the mode (`n e s m u c a d o`).
- `DirEntry` gains `links: u32`, `owner: Option<String>`,
  `allocated: FileSize`, `created: Modified`, `accessed: Modified`,
  `attributes: Attributes` (a newtype rendering `drwxr-xr-x` on Unix and
  `RHSA` letters on Windows), `description: Option<String>` (from the
  directory's description file, read once per listing by `directory.rs`).
  Owner names resolve through `libc::getpwuid_r` on Unix, cached per uid per
  listing; `None` elsewhere.
- `src/models/overlay.rs`: new variants, each holding a type owned by the
  feature's own model file under `src/models/`:

  `WipeConfirm(Vec<PathBuf>)`, `ArchivePrompt(Box<ArchivePrompt>)`,
  `ExtractPrompt(Box<ExtractPrompt>)`, `ArchiveCommands(ArchiveMenu)`,
  `Attributes(Box<AttributesDialog>)`, `ApplyCommand(ApplyPrompt)`,
  `Describe(DescribePrompt)`, `FindFile(Box<FindFile>)`,
  `CommandHistory(HistoryList)`, `FindFolder(Box<FolderTree>)`,
  `ViewHistory(HistoryList)`, `FolderHistory(HistoryList)`,
  `UserMenu(Box<UserMenu>)`, `Associations(Box<AssociationList>)`,
  `FolderShortcuts(ShortcutList)`, `Filter(FilterPrompt)`,
  `Screens(ScreenList)`, `Tasks(Box<TaskList>)`, `Hotplug(Box<DeviceList>)`,
  `SortModes(SortMenu)`, `Settings(Box<SettingsDialog>)`.

  `NamePurpose::Link` and `LinkType::Hard` extend the existing name prompt
  for Alt+F6 (§5 B1) — no new variant.
- `src/models.rs`: `App` gains `panels_shown: PanelsShown` (`Both | LeftOnly
  | RightOnly`), `settings: Settings`, `view_history: ViewHistory`,
  `folder_history: FolderHistory`, `pending: Option<Run>`. `models.rs` is
  near the line cap: move `SortMode` and friends into `models/panelview.rs`
  to stay under 1000.
- `src/config.rs` (new): `Settings` — one struct per Options page, all
  `serde` + `Default`, saved as JSON at
  `${XDG_CONFIG_HOME:-~/.config}/ffm/settings.json` (Windows:
  `%APPDATA%\ffm\settings.json`). `Settings::load()` never fails (defaults on
  any error); `save()` writes atomically. Loaded in `bootstrap.rs` like
  history, never in `App::new()`. Page structs and their fields are fixed
  by §5 D so that the dialog agents and the consumer agents agree.
- `src/models/histories.rs` (new): `ViewHistory` and `FolderHistory` — bounded
  lists (Far's default 512 → 200 here), persisted beside the command history
  (`view_history`, `folder_history`), recorded by `ops/viewer.rs`,
  `editor.rs` and `Panel::navigate_to`, in that order of ownership.

### 3.4 Dispatch, popup, rendering, module wiring

- `application/dispatch.rs`: `apply_action` arms for the four new families
  delegate to `crate::ops::<feature>::open(app)` or `run(app)`; one function
  per family to stay under the line limit.
- `popup.rs`: one arm per new `Overlay` variant → `popup::<feature>::process`.
- `ui/overlays.rs`: one arm per new variant → `ui::<feature>::render`.
- `ui.rs`: panel drawing dispatches on `panel.kind` (`Files` → existing
  renderer, `Tree` → `ui::tree::render`, `QuickView` →
  `ui::quickview::render`) and on `app.panels_shown` (a hidden panel gives its
  half to the other; a fullscreen `PanelView` on the active panel does the
  same for that frame).
- `lib.rs` declares every new module up front. Feature agents never edit
  `lib.rs`, `models/overlay.rs`, `popup.rs`, `ui/overlays.rs`, `action.rs`,
  `menu.rs`, `application/dispatch.rs`, `ui.rs` or `models.rs`; if a feature
  needs a change there it is reported back and applied by the integrator.

### 3.5 Stubs

For every feature the scaffold creates the files in §5's ownership column
with a `//! Purpose:` header, a `Default` state type, a `process` that closes
on Esc/F10 and consumes everything else, a `render` that draws the dialog
frame with Far's title and one line "Coming in this release", and an `open`
in `ops` that sets the overlay. Each stub has one test: opening sets the
variant, Esc closes it.

## 4. Waves

| Wave | Content | Parallelism |
|---|---|---|
| 0 | Scaffold (§3), then `config.rs` + `DirEntry` extension + sort modes | 2 agents, sequential |
| 1 | Left/Right rows (A) and Files rows (B) | worktrees, one agent per feature |
| 2 | Commands rows (C) | worktrees |
| 3 | Options pages (D) and the consumers of each setting | worktrees, grouped |
| 4 | Docs (CLAUDE.md tables, KB parity, F1 help, README), final gate, commit | sequential |

Integration after each wave: merge branches into main, run
`scripts/lint.sh` (with `python3`), `cargo test --features test-helpers`,
the BDD suites, fix, commit.

## 5. Features

Ownership lists the files a feature may create or edit. `M`, `P`, `U`, `O`
stand for `src/models/`, `src/popup/`, `src/ui/`, `src/ops/`; tests live in
`tests*.rs` children of those files. Every feature updates
`docs/knowledge-base/09-far-manager-compatibility/hotkeys-and-parity.md`
(its own rows only) and adds a BDD scenario when it is user-visible.

### A. Left / Right menu

**A1 View modes (Ctrl+1 … Ctrl+0).** `Panel.view` selects one of ten column
layouts, rendered by `U panel/columns.rs` (new; `U panel.rs` may be edited by
this agent only). Brief: three name columns; Medium: two; Full: name │ size │
date │ time (today's); Wide: name │ size; Detailed (fullscreen): name │ size │
allocated │ modified │ created │ accessed │ attributes; Descriptions: name │
description; Long descriptions (fullscreen): name │ size │ description; File
owners: name │ size │ owner; File links: name │ size │ links; Alternative
full: name │ formatted size │ date. Multi-column modes fill column by column;
Up/Down move within the flow, PgUp/PgDn by a screen, Home/End as now. A
fullscreen mode on the active panel takes the whole panel area. The menu ticks
the mode in use. Owner: `M panelview.rs`, `U panel.rs`, `U panel/columns.rs`.

**A2 Tree panel (Ctrl+T).** Toggles `Panel.kind` between `Files` and
`Tree`. The tree is rooted at the filesystem root of the panel's path,
expanded lazily one level at a time; Enter/Right expands or enters, Left
collapses, Up/Down move, typing quick-searches names. With
`Settings.tree.auto_change_folder` (default on) the other panel follows the
folder under the cursor. Enter on a folder turns the panel back into a file
panel there. Owner: `M tree.rs`, `U tree.rs`, `P tree.rs`, `O tree.rs`.
Shared with C4 through `M tree.rs::FolderTree`.

**A3 Quick view (Ctrl+Q).** Toggles `Panel.kind` to `QuickView`: the panel
shows the file under the other panel's cursor (first screen of text through
`ui/code.rs`, with the encoding rules of the viewer), or for a directory its
size summary from `crate::stats`. Owner: `M quickview.rs`, `U quickview.rs`,
`P quickview.rs`, `O quickview.rs`.

**A4 Sort modes (Ctrl+F12, Ctrl+F3 … Ctrl+F11).** A drop-down listing the
nine modes with the current one ticked, plus "Reverse order" (Far: choosing
the current mode again reverses). Direct keys set the mode. Owner:
`M sortmenu.rs`, `U sortmenu.rs`, `P sortmenu.rs`, `O sortmenu.rs`.

**A5 Show long names (Ctrl+N).** `Panel.long_names` toggles; when off,
names render in Far's 8.3 form (`LONGNA~1.EXT`, unique within the listing,
computed by `M shortname.rs`). Ticked while on. Owner: `M shortname.rs`,
`O longnames.rs` (the toggle), a hook in `U panel/columns.rs` reached through
one function the A1 agent leaves for it: `fn display_name(entry, panel)`.

**A6 Panel On/Off (Ctrl+F1 / Ctrl+F2).** `App.panels_shown`; hiding the
active panel makes the other active. Ticked while shown. Owner:
`O panels.rs`; rendering hook is in the scaffold (§3.4).

### B. Files menu

**B1 Link (Alt+F6).** The name prompt opens with purpose `Link`: Name (the
link to create, in the other panel by default), Link type (symbolic ⇄ hard —
hard only offered for files), Target (the entry under the cursor, absolute).
Owner: `O link.rs`, `P name_prompt.rs`, `M overlay.rs` only for
`NamePurpose::Link`/`LinkType::Hard` (pre-added by the scaffold).

**B2 Wipe (Alt+Del).** Confirmation like F8's but titled "Wipe"; on Yes each
file is overwritten with zeros, truncated, renamed to a temporary name and
deleted; directories recurse. Owner: `O wipe.rs`, `P wipe.rs`, `U wipe.rs`.

**B3 Add to archive (Shift+F1).** Dialog: archive name (default
`<name>.tar.gz`, created in the other panel), format radio (tar.gz, tar,
zip — zip only when `zip` is on PATH), "Delete files after archiving".
Runs `tar`/`zip` through `ops/shell`, reports in the status bar and the
console transcript, refreshes. Owner: `M archive.rs` (shared with B4/B5),
`P archive_prompt.rs`, `U archive_prompt.rs`, `O archive.rs`.

**B4 Extract files (Shift+F2).** Dialog: destination (other panel), for every
selected archive (`.tar`, `.tar.gz`, `.tgz`, `.tar.xz`, `.tar.bz2`, `.zip`);
`tar xf` / `unzip`. Owner: `P extract_prompt.rs`, `U extract_prompt.rs`,
`O archive.rs` (shared function set), `M archive.rs`.

**B5 Archive commands (Shift+F3).** Far's list adapted: Test archive, List
contents (opens the viewer on the listing), Extract here, Extract to other
panel, Add selected to this archive. Owner: `M archive.rs`, `P archive_commands.rs`,
`U archive_commands.rs`, `O archive.rs`.

**B6 File attributes (Ctrl+A).** Dialog for the selection: owner and group
(read-only), nine permission checkboxes (+ set-uid/gid/sticky), modification
and access time fields, "Process subfolders", Set/Cancel. Windows: Read-only,
Hidden, System, Archive. Applies to every marked entry. Owner:
`M attributes.rs`, `P attributes.rs`, `U attributes.rs`, `O attributes.rs`.

**B7 Apply command (Ctrl+G).** Prompt for a command with Far's substitutions
(`!.!` name, `!` name without extension, `!~` extension, `!\` path, `!/`
current folder, `!&` all selected names, `!@` a list file); runs once per
selected entry through `ops/shell`, output to the console transcript.
Owner: `M apply.rs`, `P apply.rs`, `U apply.rs`, `O apply.rs`.

**B8 Describe files (Ctrl+Z).** Prompt for the description of the entry under
the cursor; writes it to the directory's description file (first name in
`Settings.descriptions.names`, default `Descript.ion`, Far's format: name,
quoted when it holds spaces, a space, the text). Owner: `M describe.rs`,
`P describe.rs`, `U describe.rs`, `O describe.rs`, and `src/description.rs`
(read/write of the file, also used by `directory.rs` for A1's Descriptions
columns — created in Wave 0 with the read half).

### C. Commands menu

**C1 Find file (Alt+F7).** Dialog: mask, "Containing text", search root
(current folder / root), case-sensitive; then a results list (name and
folder) with Enter → go to the file, F3 view, F4 edit, Esc back. The walk
is bounded (200 000 entries or 10 s) and says when it stopped early.
Owner: `M findfile.rs`, `P findfile.rs`, `U findfile.rs`, `O findfile.rs`.

**C2 History (Alt+F8).** The command history as a list, newest last; Enter
runs the line, Ctrl+Enter puts it on the command line, Del clears the
history after confirmation. Owner: `M histlist.rs` (`HistoryList`, shared
with C5/C6), `P history_list.rs`, `U history_list.rs`, `O history_list.rs`.

**C3 Video mode (Alt+F9).** Far switches between 25 and 50 lines; here the
terminal is asked to resize between 25 and 50 rows (`crossterm SetSize`),
and the status bar reports the size it actually got. Owner: `O videomode.rs`.

**C4 Find folder (Alt+F10).** The folder tree as a dialog with typed quick
search; Enter sends the active panel there. Owner: `P findfolder.rs`,
`U findfolder.rs`, `O findfolder.rs`; tree model from A2.

**C5 File view history (Alt+F11).** Files opened by F3/F4, newest last;
Enter views, F4 edits, Shift+Enter re-opens without moving the item to the
end. Owner: `P view_history.rs`, `U view_history.rs`, `O view_history.rs`.

**C6 Folders history (Alt+F12).** Folders visited; Enter goes there.
Owner: `P folder_history.rs`, `U folder_history.rs`, `O folder_history.rs`.

**C7 Compare folders.** Marks, in both panels, every file with no namesake on
the other side or a different size or modification time; status reports the
count. No dialog. Owner: `O compare.rs`.

**C8 Edit user menu / F2 user menu.** The user menu lives at
`<config>/ffm/menu.ini` in Far's `FarMenu.ini` format (`H: Label` heading
lines, command lines beneath, `--` separators); F2 shows it and runs the
chosen command through `ops/shell` with B7's substitutions; "Edit user menu"
opens the file in the built-in editor, creating a commented default first.
Owner: `src/usermenu.rs`, `M usermenu.rs`, `P usermenu.rs`, `U usermenu.rs`,
`O usermenu.rs`.

**C9 File associations.** `<config>/ffm/associations.ini`: sections of mask
and the commands for Enter, F3, F4, Alt+F3, Alt+F4. The dialog lists them
(Ins/Del/F4 edit the file in the editor). `Navigation::Open` on a
non-directory consults them before the executable rule. Owner:
`src/associations.rs`, `M associations.rs`, `P associations.rs`,
`U associations.rs`, `O associations.rs`, plus the hook function the
scaffold leaves in `application/dispatch.rs::open_panel_entry`.

**C10 Folder shortcuts.** The ten bookmark slots as a dialog: path per slot,
Enter goes, Ins sets to the active folder, Del clears. Persisted in
`Settings.shortcuts`. Owner: `P shortcuts.rs`, `U shortcuts.rs`,
`O shortcuts.rs`, `M shortcuts.rs`.

**C11 File panel filter (Ctrl+I).** Dialog: include masks, exclude masks,
"Hide dotfiles"; applies to the active panel's listing (`Panel.filter`),
the panel title shows `*` while a filter is on. Owner: `M filter.rs`,
`P filter.rs`, `U filter.rs`, `O filter.rs`; the filtering call is in
`Panel::load_entries` (scaffold leaves `filter.apply(&mut entries)`).

**C12 Screens list (F12).** Lists the screens this program has: Panels and
Console; Enter switches. Owner: `M screens.rs`, `P screens.rs`,
`U screens.rs`, `O screens.rs`.

**C13 Task list (Ctrl+W).** The processes this program started that are
still running (children of the process, read from `/proc` on Linux and
`ps` elsewhere on Unix; unavailable on Windows with a status line saying
so); Del asks and terminates. Owner: `M tasks.rs`, `P tasks.rs`, `U tasks.rs`,
`O tasks.rs`.

**C14 Hotplug devices list.** Removable block devices on Linux
(`/sys/block/*/removable`, mount points from `/proc/mounts`); Del unmounts
after confirmation. Elsewhere the row reports that the list is not available
on this platform. Owner: `M hotplug.rs`, `P hotplug.rs`, `U hotplug.rs`,
`O hotplug.rs`.

### D. Options menu

One dialog engine, one page table per row. `SettingsDialog` holds the page,
its rows (`Toggle(bool)`, `Text(String)`, `Number(u32)`, `Choice(index, &[&str])`),
the focused row and the OK/Cancel focus; Space flips a toggle, typing edits
text, Left/Right step a choice, Enter on OK writes the rows back into
`Settings` (in memory — Far saves on "Save setup" or when
`system.auto_save_setup` is on, at exit). Owner of the engine:
`M settings_dialog.rs`, `P settings.rs`, `U settings.rs`. Each page is a
`fn rows(&Settings) -> Vec<SettingRow>` and `fn apply(rows, &mut Settings)`
in `src/config/pages/<page>.rs`, owned by the page's agent, who also wires
the consumer of every setting the page shows. Every setting listed here is
consumed somewhere; a setting nothing reads is not added.

| Page | Settings (field: consumer) |
|---|---|
| System | `auto_save_setup: bool` (bootstrap saves at exit); `wipe_on_delete: bool` (F8 wipes, B2); `history_size: u32` (command history cap) |
| Panel | `show_hidden: bool` (dotfiles, `Panel::load_entries`); `dirs_first: bool`; `case_sensitive_sort: bool`; `show_column_titles: bool`; `show_status_line: bool` (the panel footer); `show_free_size: bool`; `highlight_files: bool` (D Highlighting on/off) |
| Tree | `auto_change_folder: bool` (A2); `min_width: u32` |
| Interface | `clock: bool` (menu bar clock); `key_bar: bool` (F-key bar); `menu_bar: bool` (always shown / only on F9); `mouse: bool` (mouse events ignored when off) |
| Languages | `main: Lang` (En/Ru: menu bar, drop-downs, key bar, dialog titles); `help: Lang` (F1 text) — translations in `src/i18n.rs` |
| Plugins configuration | list of MCP servers from `.mcp.json` with `disabled: Vec<String>` (F11 status marks them) |
| Plugins manager | `scan_project_mcp: bool` (read `.mcp.json` from the panel's folder for F11); `show_status_on_start: bool` |
| Dialog | `remember_history: bool` (name/mask prompts keep last values); `click_outside_closes: bool` |
| Menu | `far_letters: bool` (hotkey letters underlined); `wrap_cursor: bool` (Up at the top wraps to the bottom, default on) |
| Command line | `prompt_format: String` (segments, `$p$g` style: `$p` path, `$b` branch, `$u` user, `$h` host, `$g` sigil); `history_size` shares System's |
| AutoComplete | `fish: bool` (B: shell out to fish); `ghost: bool` (grey suggestion); `menu: bool` (drop-down on Tab) |
| InfoPanel | `show_kinds: bool`, `show_largest: bool`, `largest_count: u32` (Ctrl+L sections) |
| Groups of file masks | `groups: Vec<(String, String)>` name → masks, edited as `name=masks;…` text; usable in Gray+/Gray− and C11 as `<name>` |
| Confirmations | `copy_overwrite, move_overwrite, delete, delete_folders, exit, clear_history: bool` (each read where the question is asked) |
| File panel modes | per `PanelView`: `columns: String` (Far's `N,S,D,T` letters) and `fullscreen: bool`, consumed by A1's layout |
| File descriptions | `names: Vec<String>` (default `Descript.ion`, `Files.bbs`); `update_on_copy: bool`; `update_on_move: bool` |
| Folder description files | `names: Vec<String>` (default `Dirinfo`, `Folder.txt`) shown in A1's Descriptions modes for directories |
| Viewer | `tab_size: u32`; `wrap: bool` (default off, Far's); `default_encoding: String` |
| Editor | `tab_size: u32`; `expand_tabs: bool`; `auto_indent: bool`; `save_position: bool` |
| Code pages | `favorites: Vec<String>` — the encodings the viewer cycles through |
| Colors | `theme: Theme` (`FarBlue`, `Black`, `Grey`) — `ui/palette.rs` becomes a function of the theme |
| Files highlighting | `rules: Vec<(String, String)>` mask → colour name, consumed by `ui/panel.rs::entry_colour` |
| Save setup (Shift+F9) | writes `Settings` to disk; status confirms the path |

### E. Menu cursor

`Menu::step` keeps skipping separators only. `Menu::open` lands on row 0.
With `Settings.menu.wrap_cursor` off, Up on the first row and Down on the
last stay put.

## 6. Acceptance

- `cargo test --features test-helpers` green; BDD suites green;
  `scripts/lint.sh` green (run `python3` for its scripts on hosts without
  `python`).
- `every_far_menu_is_present_in_full` counts unchanged; a new test asserts
  every row is selectable and every `Run::Key` resolves to something other
  than `QuickSearchClear`.
- Manual smoke in tmux: F9, walk every heading with Left/Right, every row
  with Up/Down, Enter on each row opens what §5 says.
- Docs: CLAUDE.md hotkey tables and the knowledge-base parity page list every
  row as ✅ with its adaptation noted; F1 help lists the new keys.
