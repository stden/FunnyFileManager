<!-- Purpose: Design for the interactive Language servers dialog: editing servers in the settings file, completion, highlighting, and a settings write that keeps every key it does not own. -->

# Language servers editor: design

_2026-09-23, revised the same day after an adversarial review (§17).
Status: implemented (T1–T13). This file is the design record. The current
behaviour is described in `docs/editor-lsp.md` and the hotkey parity doc, and
§18 lists where the build departed from the plan. The dialog was built when
the settings were `settings.json`; §21 records how it was ported onto
`settings.yaml`, and which parts of §10 and §12 that superseded._

## 1. What the user asked for

"Add highlight and autocomplete to this dialog", where the dialog is
**F9 → Commands → Language servers (LSP)** and the `lsp` command. Asked what
autocomplete should mean, the user chose server editing:

1. The dialog becomes interactive. The user selects a language server, edits
   its command, arguments and file extensions, and toggles it on or off.
2. Autocomplete offers commands found on `PATH`, the known server binaries,
   common arguments such as `--stdio`, and extensions.
3. Changes are saved to `settings.json` in the platform config directory
   (`~/.config/ffm/settings.json` on Linux, `%APPDATA%\ffm\settings.json` on
   Windows).
4. The report is highlighted: headings, `[enabled]` / `[disabled]`, status
   values and the auto-install JSON.

Standing bar: as reliable as possible; enums and newtypes instead of bool and
string flags; KISS and DRY; every repository gate (clippy all + pedantic +
nursery, module boundaries, 1000-line files, public-API coverage, tests in
sibling files, no network in tests, the Far hotkey parity doc).

## 2. Facts established

Checked against the tree at `d78f47c`.

- **The report.** `ops::lsp_status::open` builds
  `Viewer::report("Language servers", crate::lsp::inventory(..))`. The lines
  are plain `String`s from `src/lsp/install/report.rs`, and
  `ui::viewer::render_viewer_body` draws every report with the plain
  highlighter. Five other reports share that path, and
  `a_report_body_is_plain_and_its_bar_has_no_file_fields` pins it.
- **Server settings.** `config::ServerSettings` has the fields
  `enabled: bool`, `command`, `args`, `extensions` (stored without the dot),
  `root_markers` and `install: Option<InstallRecipe>`. `LspSettings` merges
  each user entry onto the embedded catalog (`src/config/lsp/catalog.json`,
  12 languages) field by field, and parses the catalog again on every
  `LspSettings::default()`. An unknown language starts from
  `ServerSettings::default()`, which is the rust-analyzer entry.
  `LspSettings::for_path` takes the first *enabled* server in `BTreeMap` order
  whose extension matches, ignoring ASCII case. A deserialize error does not
  name the language (`config/lsp.rs:55-61`).
- **The runtime.** `application::lsp_jobs` reads `app.config.lsp` only while
  an editor is open. With no editor overlay, `observe` closes the session
  (`lsp_jobs.rs:43-51`). Overlays do not stack, so no server runs while this
  dialog is open, and the next file opened in the editor starts its server
  with whatever `app.config.lsp` then holds (`worker/lifecycle.rs:38-45`).
- **Resolution falls back to the recipe.** `report.rs::resolve` tries the
  configured command first. When the command is not found and the server has
  an `install` recipe, it reports the cached install (`report.rs:97-118`). The
  runtime does the same: `NotFound` plus `install.is_some()` starts the
  installer (`worker/lifecycle.rs:105-111`), and the receipt's command
  replaces the configured one (`launch/receipt.rs:41`). A catalog server whose
  command was changed to a program that is not installed therefore runs the
  catalog's program instead.
- **Two writers already share `ffm/settings.json` with different schemas:**
  - `ops::menu::settings::save` (Shift+F9 and F9 → Options → Save setup)
    writes only the `SettingsPage` map (`{"Editor": "8", …}`). It erases
    `lsp` and every `config::Settings` page.
  - At startup, `ops::menu::settings::load` reads the file as
    `BTreeMap<SettingsPage, String>`. A file that holds `lsp` fails with
    "unknown variant". The load is all or nothing: `app.settings` is replaced
    only when every page parses, validates and applies
    (`ops/menu/settings.rs:52-71`). On any failure, `load_default` only sets a
    status, and `app.settings` keeps the default `Persistence::SessionOnly`.
  - `config::Settings::save` serializes the whole typed struct. That replaces
    each page object whole, drops unknown keys and the `SettingsPage` keys,
    pins every catalog entry into the file, and writes through
    `write_owner_only`, which replaces a symlink. In production only the exit
    auto-save (when `auto_save_setup` is on) reaches it. `ops::savesetup::run`
    calls it too, but Shift+F9 resolves through `action::menu::resolve` first
    (`action.rs:125-130`, `action/menu.rs:29`). So
    `Action::Tool(ToolCommand::SaveSetup)` (`function_keys.rs:68`) is reached
    only by a test.
  - `config::Settings::load` turns a file that does not parse into the
    defaults and only logs the error. It has no guard against a FIFO or an
    oversized file.
- **Paths.** The two readers resolve the path differently.
  `config::Settings::path()` uses `%APPDATA%` on Windows, otherwise an
  absolute `$XDG_CONFIG_HOME`, otherwise `~/.config`.
  `ops::menu::settings::load_from_environment` tries `APPDATA` first on every
  OS and accepts a relative `XDG_CONFIG_HOME`.
- **AutoComplete exists twice.** `SettingsPage::AutoComplete` (`"on"` /
  `"off"`, F9 → Options) gates the command line's Tab completion and ghost
  text (`popup/command_line.rs:383-388, 490-496`). Its caption says
  "Command-line completion and suggestions". The typed
  `config::Settings.auto_complete` (`fish`, `ghost`, `menu`) is read by
  nothing. The command line opens its list only on Tab, and refreshes an open
  list whatever the setting (`popup/command_line.rs:451-465`).
- **Host state.**
  - `lsp::install::cache_dir()` is `config::dir()/lsp`, the real
    `~/.config/ffm/lsp`.
  - `report.rs` finds programs through `System`, which reads the process
    `PATH` and `JAVA_HOME`.
  - `complete::execs::executables()` is a process-wide `OnceLock` over the
    real `PATH`. The scan calls `fs::metadata` for every entry of every `PATH`
    directory (`execs.rs:44-86`), which takes seconds on WSL's `/mnt/c/…`
    entries.
  - On Windows every file counts as executable (`execs.rs:81-85`), while LSP
    lookup accepts only `<name>.exe` (`launch.rs:212-220`).
- **Visibility.** `lib.rs` declares `pub(crate) mod lsp`. `lsp.rs`
  declares `mod install` privately and re-exports only `install::inventory`.
- **serde_json.** A test build has `preserve_order` through a dev-dependency
  (`aws-smithy-http-client`). A release build does not. With
  `preserve_order`, `Map::remove` is `swap_remove`, which moves the last key
  into the removed slot (`serde_json-1.0.151/src/map.rs:150-165`), and
  `shift_remove` keeps the order.
- **Mouse.** While an overlay is open, the wheel becomes Up/Down
  (`dialog_mouse.rs:51-58`). A click on a button drawn by
  `dialog::render_tracked` becomes the key that `dialog_button_key` names for
  that overlay (`button_keys.rs:18-60`). An overlay without an arm ignores
  clicks. Only the panels' key bar is hit-tested (`mouse.rs`).
- **Patterns to reuse:**
  - `src/path_env.rs`, `popup/path_env.rs`, `ops/path_env.rs` and
    `ui/path_env.rs` form an editable list with a
    Browse / Edit / ConfirmSave / ConfirmDiscard mode, a `save_with(.., write)`
    test seam and a revision check. It has no undo, and Del/F8 removes at
    once.
  - `ops/editor.rs::resolve_save_target` writes through a symlink. It is
    `canonicalize().unwrap_or(path)`, so a dangling link falls back to the
    link itself. The editor writes in place when a file has other hard links
    or another owner (`rewrite_in_place`, `ops/editor.rs:98-100`), giving up
    atomicity.
  - `file_write::{temp_beside, persist_over}` keep the target's permissions.
    `file_write::ensure_writable` refuses a file the user cannot write before
    a rename could replace it.
  - `ops::menu::settings::read_json` accepts only a regular file of at most
    1 MiB.
  - `ui::dialog::{Row, Button, render_tracked, field_caret}` draw Far forms.
    `Row::Field` draws its box and Far's `↓` list marker through
    `captioned_box`, as `Row::Choice` does (`dialog.rs:534, 689`).
  - `cycle::step` gives a tab order. `models::ExitChoice` and the editor's
    exit dialog answer Save / Discard / Cancel.
  - `models::{Menu, Candidate}` hold the completion list, and `ui/completion.rs`
    draws it. A `Menu` cannot be empty and always has a selected row.
  - `complete::{execs, matcher}` provide the `PATH` scan and the ranking.
  - `highlight::Highlighter` knows JSON, and `ui::code::token_style` maps its
    tokens to colours.
  - The drive dialog writes its key legend into the bottom border.
- **Test harness:**
  - `models::settings::Settings::with_file(path)` injects the settings file.
    `App::with_panels` is `SessionOnly` by default.
  - `complete::execs::scan(Some(..))` is the `PATH` seam.
  - The TUI BDD (`tests/tui_bdd`) runs scenarios serially with
    `fail_on_skipped`. `Session::inventory` writes an absolute fixture server.
  - `test_support` is compiled under `test-helpers`, so it can set
    `pub(crate)` state for the BDD crate.

## 3. Decisions

| #   | Decision                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Why, and what was rejected                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1  | The report Viewer is replaced by one overlay, `Overlay::LspEditor`: a server list, a highlighted detail pane for the selected server, and an F3 view of all servers.                                                                                                                                                                                                                                                                                                                                                                                                                                              | Rejected: highlighting `ViewSource::Report` by parsing text. It would re-read label literals, touch six `Viewer::report` callers and recolour the MCP report. Also rejected: a Viewer plus a separate editor, because overlays do not stack and Esc from the Viewer would lose the draft.                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| D2  | The detail is drawn from typed data (`ServerSettings` + `Availability`), never from text. `lsp::inventory` and its `Vec<String>` report are deleted.                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Highlighting from types cannot drift from wording. Rejected: a `LineMode::LspReport` scanner in `highlight.rs`, which is brittle and duplicates the labels.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| D3  | Every top-level key of `settings.json` has exactly one owner. `lsp` belongs to this dialog. The `SettingsPage` names (`"Editor"`, …) belong to Save setup. The other `config::Settings` fields (`system`, `panel`, …) belong to `config::Settings::save`. Each writer does a read-modify-write of the raw JSON object through one shared module, `config::document`. It sets only its own keys and never removes a key it does not own. Save setup writes only the pages it holds a value for. `config::Settings::save` merges its pages key by key. The dead `ops::savesetup::run` route is deleted, not ported. | This is the Rule of Three: three writers, one mechanism. Rejected: leaving Shift+F9 as it is, because the next Shift+F9 would erase every LSP edit and the next start would report "unknown variant `lsp`". Also rejected: porting `ops::savesetup::run`, which no key reaches.                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| D4  | Inside `lsp`, a save is a **patch of owned fields** (`enabled`, `command`, `args`, `extensions`). Only fields whose draft differs from the baseline are written. A catalog server whose field returns to the catalog value has that field removed, not pinned. The save is refused when the file's raw `lsp` subtree differs from the one read at open (`LspChanged`).                                                                                                                                                                                                                                            | Partial overrides stay partial, catalog upgrades keep reaching the user, and `root_markers`, `install` (including an explicit `null`) and unknown fields are never touched. After D3, no other FFM writer touches `lsp`. The subtree check therefore refuses only when another program, or another FFM's dialog, edited `lsp` while this one was open. Edits outside `lsp` never block. Rejected: serializing `LspSettings`, which pins the jdtls URL and hash. Rejected: a whole-file revision (the PATH editor's rule), which refuses safe saves. Rejected: per-field compare-and-set (this spec's first draft). It saves in a few more cases but needs an expected value per change and a list of conflicting paths.                  |
| D5  | The target file is `app.settings.persistence.file()`. `ops::menu::settings::load` sets it to the file **before** it parses anything. A failed Options load (bad JSON, `"Editor": "20"`, a missing Language dictionary, a `refresh_panels` error) therefore still leaves the location known. `load_default` resolves the path with `config::Settings::path()`.                                                                                                                                                                                                                                                     | There is one path rule, and one place that holds the result. Tests and BDD inject the file with `Settings::with_file`. A `SessionOnly` app has no file, so its dialog is read-only. The dialog reads and checks the file itself (§10.3), so it opens writable, or read-only with the real reason, whatever happened to the Options pages. This is safe for Save setup because it writes only the pages in `values()` (D3), and after a failed load those are only the pages set in this session. Rejected: reading `config::Settings::path()` in the dialog, which forces environment mutation in tests and adds a second path rule. Also rejected: a separate `settings_file` field on `App`, which would be a second copy of one fact. |
| D6  | `serde_json` gets `features = ["preserve_order"]` in `[dependencies]`. Every key removal uses `shift_remove`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | The release build then behaves like the tested build, and a save keeps the user's key order. `Map::remove` is `swap_remove` under that feature and would reorder the user's keys. Before the change, audit with `rg` every place in `src` that iterates a `serde_json::Map` or compares serialized JSON. `cargo tree` only shows who turns the feature on. The suite already runs with the feature, so it pins the order-dependent behaviour it tests.                                                                                                                                                                                                                                                                                   |
| D7  | Arguments are edited on one line with a small, platform-neutral grammar. Whitespace outside quotes separates arguments. `"` opens or closes a quoted run anywhere in an argument, as in a shell, so `--x="a b"` is `--x=a b`. Inside a run, `""` is a literal quote. There are no backslash escapes, and a control character is refused.                                                                                                                                                                                                                                                                          | Servers run without a shell (`Command::args`), so shell quoting is the wrong language. Backslashes must survive for Windows paths. A control character cannot be shown in a one-line field, and a NUL makes `Command::args` fail at spawn. Rejected: `complete::shellwords`, whose POSIX escapes turn `C:\work` into `C:work`. Also rejected: one row per argument, a second list inside a form for a rare case.                                                                                                                                                                                                                                                                                                                         |
| D8  | When the AutoComplete setting is on, typing opens a **passive** list: nothing is highlighted, Enter still presses OK, and Down or Up moves into the list. The setting is `SettingsPage::AutoComplete`, the F9 → Options page that already governs command-line completion. Ctrl+Space, Ctrl+Down or Alt+Down open an **active** list, with its first row highlighted, whatever the setting. Every edit refilters an open list, whatever the setting. Tab keeps its Far meaning (next control). The page's caption becomes `Command-line completion and suggestions; dialog lists while typing: on/off`.           | Far's AutoComplete shows a list without taking Enter away from the dialog. Ctrl+Space is the built-in editor's completion key. Ctrl+Down is Far's "open this field's list". Alt+Down is the Windows combo-box key, and it is the one of the three that macOS does not take by default (§5.7). Rejected: Tab for completion, which clashes with Far form navigation. Rejected: auto-opening with the first row highlighted, because Enter would stop meaning OK while the user types. Also rejected: the typed `config::Settings.auto_complete`, which nothing reads or edits.                                                                                                                                                            |
| D9  | A command that is not on `PATH` is a warning, never a block. The Status row says what FFM will do: run the command, install after workspace trust, do nothing, or run the auto-install recipe's program **instead** of a changed command (§11, the `recipe` tag in amber).                                                                                                                                                                                                                                                                                                                                        | The user may install the server after saving. The runtime's recipe fallback (§2) must never be shown as `ready` for a command it will not run.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| D10 | The write resolves a symlink to the file it names and refuses a dangling one. It checks `file_write::ensure_writable`, and on Unix it refuses a file that has other hard links or another owner. It keeps the file's permissions, renames atomically, syncs the directory on Unix and refuses when the file changed while saving. Dialog saves, and only they, keep `settings.json.bak` beside the link.                                                                                                                                                                                                          | This makes it hard to lose the user's file. Settings writes must be atomic, so the editor's in-place fallback for hard links and foreign owners is not available. Refusing is better than silently detaching the other names or taking the file over. The `.bak` sits in the config directory, where the docs say it is. An exit auto-save cannot overwrite the copy a user would restore. Rejected: replacing a symlink as `write_owner_only` does, which breaks dotfile repos. Rejected: canonicalize-or-path, which turns a dangling link into a regular file. Rejected: writing in place, which is not atomic.                                                                                                                       |
| D11 | Not in this change: editing `root_markers` or `install` (including an Auto-install checkbox), renaming a language, undo, clicks on list rows or on the dialog's key bar, and a Help topic.                                                                                                                                                                                                                                                                                                                                                                                                                        | This keeps the scope to what was asked. `"install": null` in the file still disables auto-install, and the `recipe` status (§11) shows when it matters. Undo buys little: Esc → No discards the whole draft, and Del resets one catalog server. The PATH editor precedent has no undo either.                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| D12 | Everything the dialog asks of the machine comes from one value on `App`, `app.lsp_probe: lsp::Probe` (§9.5): the LSP install cache, the `PATH` and `JAVA_HOME` used to find programs, and the program names offered for Command. `App` starts with an empty probe (no cache, no `PATH`, no programs). Bootstrap installs `Probe::from_process()`, as it installs the clipboard.                                                                                                                                                                                                                                   | Availability and completion become deterministic in ops tests and BDD, which inject a TempDir cache and a scanned fake `PATH` through `test_support`. No test reads the real `~/.config/ffm/lsp`. Rejected: passing `System`, `cache_dir()` and `executables()` as parameters of `open`, because BDD reaches `open` through keys and cannot pass them.                                                                                                                                                                                                                                                                                                                                                                                   |

## 4. Screen layout and key bar

The dialog is a full-screen mode like the viewer and the editor, drawn on the
panel background (`palette::BACKGROUND`), where the code colours are designed
to read. The window is framed. The bottom border carries a legend for the keys
that are not F-keys (the drive dialog's `FOOTER` pattern). The last screen row
is the dialog's own F-key bar in Far's 12-cell layout, drawn by the key-bar
renderer with labels passed in (see §15, T10).

### 4.1 List mode (the dialog opens here), 80×24

```
╔══════════════════════════════ Language servers ══════════════════════════════╗
║ Settings: /home/user/.config/ffm/settings.json                               ║
╟──────────────────────────────────────────────────────────────────────────────╢
║   html               [x] vscode-html-language-server .html .htm        ready ║
║   java               [x] jdtls                       .java              auto ║
║   javascript         [x] typescript-language-server  .js .mjs .cjs      auto ║
║   javascriptreact    [x] typescript-language-server  .jsx               auto ║
║   json               [x] vscode-json-language-server .json .jsonc      ready ║
║ ▸ python          *  [x] pylsp                       .py .pyi          ready ║
║   rust               [ ] rust-analyzer               .rs             missing ║
║   shellscript        [x] bash-language-server        .sh .bash          auto ║
╟──────────────────────────────────────────────────────────────────────────────╢
║ python  [enabled]  .py .pyi                                                  ║
║   Command:          pylsp                                                    ║
║   Arguments:        (none)                                                   ║
║   Status:           installed in FFM cache                                   ║
║   Executable:       /home/user/.config/ffm/lsp/7c1e…/venv/bin/pylsp          ║
║   Launch arguments: (none)                                                   ║
║   Auto-install:     {"kind":"python","package":"python-lsp-server[pyfl       ║
║                     akes,pycodestyle]","module":"pylsp"}                     ║
╟──────────────────────────────────────────────────────────────────────────────╢
║ Modified: python. F2 saves only what changed.                                ║
╚══ Enter Edit  Space On/Off  Ins Add  Del Reset  Esc Close ═══════════════════╝
1      2Save  3View  4Edit  5Check 6      7Add   8Reset 9     10Quit11    12
```

- **Row 1** shows the settings path. In read-only mode it reads
  `Settings: <path> — read only: <reason>`, or
  `Settings: none for this session — read only` for a `SessionOnly` app.
- **List rows**, 76 columns inside the frame, in this order:
  - the marker `▸ `;
  - the language (16 columns, cut with `…`);
  - a two-column mark column: `*` when the draft differs from what was
    loaded, `!` when the server has a problem (§8);
  - the checkbox `[x]` or `[ ]`;
  - the command (27 columns, cut with `…`);
  - the extensions (15 columns);
  - the availability tag (7 columns, right-aligned): `ready`, `auto`,
    `recipe`, `missing` or `error` (§11).

  The list scrolls to keep the selection visible, and its height is
  `min(servers, body / 2)`.
- **Detail pane**: the selected server's detail (§6), wrapped to the width.
  When it does not fit, its last row reads `… F3 shows everything`.
- **Message row**: the notice the last key set (§9.3), in `Info` or `Error`
  colours. A notice lasts one key: the next key clears it before it acts, so a
  `Saved to …` never stands over changes made after the save. With no notice,
  the row gives the read-only reason, the dirty summary or `No changes`.
- **Bottom-border legend**: in list mode it names the server-specific key:
  `Del Reset` on a catalog server, `Del Remove` on an added one. The F8 label
  follows the same rule (`8Reset` / `8Remove`).
- **Key bar**: `2Save 3View 4Edit 5Check 7Add 8Reset|8Remove 10Quit`. The
  other cells are empty, because a label for a key that does nothing is worse
  than none. In read-only mode, 2, 4, 7 and 8 are empty. The bar is drawn, not
  hit-tested (§5.6).
- **Small screens**: below 16 rows the detail pane is hidden, and the message
  row says `F3 shows details`. Below 60 columns the command column shrinks
  first, then the extensions. A window too short for every row keeps the
  settings row, then the message row under it; the rules and the list give
  way first, and no row is drawn over another (§20).

### 4.2 Edit form (Enter / F4 on a server, Ins / F7 for a new one)

This is a standard Far grey dialog (`dialog::render_tracked`,
`Ground::Prompt`) over the list. The suggestion list is drawn *over* the rows
below the field. It is shown expanded here so it can be read. Opened by
typing, it is passive and no row is highlighted (§7.3).

```
╔══════════════════ Edit server: python ═══════════════════╗
║ Language:    [python                            ]↓       ║
║ Command:     [py▏                               ]↓       ║
║              ┌──────────────────────────────────┐        ║
║              │pylsp                      catalog│        ║
║              │pydoc3                        PATH│        ║
║              │python3                       PATH│        ║
║              │pyright-langserver            PATH│        ║
║              └──────────────────────────────────┘        ║
║ Arguments:   [                                  ]↓       ║
║ Extensions:  [.py .pyi                          ]↓       ║
║ [x] Enabled                                              ║
╠══════════════════════════════════════════════════════════╣
║ Program name on PATH, or a full path. No shell runs it.  ║
╠══════════════════════════════════════════════════════════╣
║                  { OK }  [ Cancel ]                      ║
╚══════════════════════════════════════════════════════════╝
```

- **Rows**:
  - `Row::Field` for Language, Command, Arguments and Extensions. Every
    captioned field draws Far's `↓` marker (`captioned_box`), so
    `ui/dialog.rs` needs no change. Language is `enabled: false`, dimmed and
    skipped, for an existing server, and editable for a new one. It has no
    list: Ctrl+Space there says `No suggestions`.
  - `Row::Checkbox` for Enabled.
  - A divider, then one text row: the help for the focused control, or the
    reason the last OK was refused.
  - The buttons `{ OK }` and `[ Cancel ]`.
- **Title**: `Edit server: <language>` or `Add server`.
- **Key bar and legend**: no F-key acts in the form, so the key bar is blank
  (a label for a key that does nothing is worse than none). The list window's
  bottom border names the form's keys: `Enter OK  Esc Cancel  Tab Next
  Ctrl+Space List`; with a passive list open `Down Choose  Enter OK  Esc Close
  list`; with an active one `Up Down Choose  Enter Insert  Esc Close list`. The
  exit question (§4.4) also blanks the key bar, and its legend reads `Y Yes  N
  No  Esc Cancel`.
- **Long values** scroll horizontally the way
  `ui::menu_dialog::render_input` scrolls (`tail_cells`). The caret column
  comes from `dialog::field_caret(caption, visible_before_caret)`. There is
  no magic column offset.
- **Help text** for each control:

  | Control    | Help row                                                    |
  | ---------- | ----------------------------------------------------------- |
  | Language   | `The LSP languageId: a-z 0-9 - _ . +`                       |
  | Command    | `Program name on PATH, or a full path. No shell runs it.`   |
  | Arguments  | `Separate with spaces; quote one that holds a space: "a b"` |
  | Extensions | `Space-separated; the dot is optional: py pyi`              |
  | Enabled    | `Space toggles`                                             |

### 4.3 View mode (F3): every server, as the old report showed it

```
╔═══════════════════════════ Language servers: all ════════════════════════════╗
║ Settings: /home/user/.config/ffm/settings.json                               ║
║ Availability is what FFM would start; it is not a process list.              ║
╟──────────────────────────────────────────────────────────────────────────────╢
║ css  [enabled]  .css                                                         ║
║   Command:          vscode-css-language-server                               ║
║   Arguments:        --stdio                                                  ║
║   Status:           not installed; installs after workspace trust            ║
║   Auto-install:     {"kind":"npm","packages":["vscode-langservers-extr       ║
║                     acted"],"package":"vscode-langservers-extracted",        ║
║                     "binary":"vscode-css-language-server"}                   ║
║                                                                              ║
║ html  [enabled]  .html .htm                                                  ║
║   Command:          vscode-html-language-server                              ║
║   Arguments:        --stdio                                                  ║
║   Status:           found configured executable                              ║
║   Executable:       /usr/local/bin/vscode-html-language-server               ║
║   Launch arguments: --stdio                                                  ║
║   Auto-install:     {"kind":"npm","packages":["vscode-langservers-extr       ║
║                     acted"],"package":"vscode-langservers-extracted",        ║
║                     "binary":"vscode-html-language-server"}                  ║
║                                                                              ║
╚══ Lines 1-19 of 97  Up Down PgUp PgDn Home End  F3/Esc List ═════════════════╝
1      2      3List  4      5Check 6      7      8      9     10Back11    12
```

The same `detail_lines` function (§6) draws the detail pane and this view,
so there is one copy of the layout and the colours.

### 4.4 Exit question (Esc or F10 with unsaved changes)

`dialog::render_warning` with the title `Language servers`, the text
`Language server settings have been modified. Save?` and the buttons `Yes`,
`No` and `Cancel`, driven by `models::ExitChoice`. This is the editor's exit
dialog. Its buttons answer clicks (§5.6).

## 5. Keymap

While an overlay is open, `application::dispatch_key` sends **every** key to
`popup::process_popup_input`. No key below can reach a panel or the command
line, so none of them clashes with an existing ffm binding. The table chooses
each key to mean what it already means in ffm or Far. Keys not listed are
ignored: F1 (no help topic), F6, F9, F11, F12, Shift+F9, Ctrl+Z and
Alt+letters. No list or view key takes a modifier, so one pressed with Shift,
Ctrl or Alt is another key and is ignored too (Shift+F8 or Alt+F7 means
something else in Far). The mouse is covered in §5.6.

### 5.1 List mode

| Key         | Action                                                                               | Precedent                          |
| ----------- | ------------------------------------------------------------------------------------ | ---------------------------------- |
| Up / Down   | Previous / next server                                                               | Far lists; `menu_dialog::navigate` |
| PgUp / PgDn | One list page                                                                        | Far lists                          |
| Home / End  | First / last server                                                                  | Far lists                          |
| Enter, F4   | Edit the selected server                                                             | PATH editor Enter/F4; F4 = Edit    |
| Space       | Toggle Enabled on the selected server                                                | Far checkbox                       |
| Ins, F7     | Add a server                                                                         | PATH editor Ins/F7                 |
| Del, F8     | Catalog server: reset its four owned fields to the catalog. Added server: remove it. | PATH editor Del/F8                 |
| F2          | Save to `settings.json` (§10)                                                        | Editor and PATH editor F2          |
| F3          | View mode: every server                                                              | Far F3 = View                      |
| F5          | Check availability again for every server                                            | PATH editor F5                     |
| Esc, F10    | Close. With changes, ask Save / Discard / Cancel.                                    | Editor exit, Far F10               |

In read-only mode, Space, Enter/F4, Ins/F7, Del/F8 and F2 only repeat the
read-only reason in the message row.

### 5.2 View mode (F3)

| Key          | Action                                         |
| ------------ | ---------------------------------------------- |
| Up / Down    | One line                                       |
| PgUp / PgDn  | One page                                       |
| Home / End   | Top / bottom                                   |
| F5           | Check availability again                       |
| F3, Esc, F10 | Back to the list, with the selection unchanged |

### 5.3 Form

| Key                                                   | Action                                                                        | Precedent                                          |
| ----------------------------------------------------- | ----------------------------------------------------------------------------- | -------------------------------------------------- |
| Tab / Shift+Tab                                       | Next / previous control, wrapping; Language is skipped for an existing server | Far dialogs; `cycle::step` over `FormField::ORDER` |
| Down / Up (no list open)                              | Next / previous control                                                       | Far dialogs                                        |
| Characters, Backspace, Delete, Left, Right, Home, End | Edit the focused field                                                        | `popup::menu_dialog::edit`                         |
| Ctrl+Space, Ctrl+Down, Alt+Down                       | Open an active suggestion list for the focused text field                     | Editor Ctrl+Space; Far Ctrl+Down; Windows Alt+Down |
| Space                                                 | On Enabled: toggle. On a button: press it. In a text field: a space.          | Far dialogs                                        |
| Enter                                                 | OK, or Cancel when Cancel has focus                                           | Far default button                                 |
| Esc                                                   | Cancel the form. Its edits are dropped and the list is unchanged.             | Far dialogs                                        |

### 5.4 Suggestion list (open inside the form)

A list opened by typing is **passive**: no row is highlighted and the form
keeps Enter. A list opened by Ctrl+Space, Ctrl+Down or Alt+Down, or entered
with Down or Up, is **active** (§7.3).

| Key                           | Passive list                                                               | Active list                                                                                |
| ----------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| Down / Up                     | Activate it on the first / last row                                        | Move the highlight, wrapping (`Menu::next` / `Menu::previous`)                             |
| Enter                         | Close the list, then the form's Enter (OK, or Cancel when focused)         | Put the highlighted suggestion in the field (§7.4)                                         |
| Esc                           | Close the list. The form stays.                                            | The same                                                                                   |
| Tab / Shift+Tab               | Close the list and move focus                                              | The same                                                                                   |
| Characters, Backspace, Delete | Edit and refilter; the list stays passive. It closes when nothing matches. | Edit and refilter; the highlight returns to the first row. It closes when nothing matches. |
| Left, Right, Home, End        | Move the caret and close the list, so a stale list cannot be accepted      | The same                                                                                   |

### 5.5 Exit question

These keys are exactly the editor's (`popup/editor.rs::confirm_exit`).

| Key                | Action                 |
| ------------------ | ---------------------- |
| Left / Right / Tab | Previous / next button |
| Enter              | The focused button     |
| Y / N              | Yes / No               |
| C, Esc             | Cancel                 |

- **Yes** saves. On success the dialog closes. On failure the dialog returns
  to the list with the error.
- **No** closes and sets `Status::ChangesDiscarded`.
- **Cancel** returns to the list.

### 5.6 Mouse

The dialog uses the shared overlay mouse paths and adds one arm to
`dialog_button_key` and one to `click_dialog_row`. It has no click handler
of its own.

| Where         | Wheel (sent as Up / Down)                                       | Click                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| ------------- | --------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| List mode     | Previous / next server                                          | Nothing. List rows and the dialog's key bar are not hit-tested (D11).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| View mode     | One line                                                        | Nothing                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| Form          | Previous / next control. With a list open, as §5.4's Up / Down. | A click on `{ OK }` or `[ Cancel ]` focuses that button, then sends Enter, so an old keyboard focus cannot turn OK into Cancel. This is the attributes dialog's rule (`attributes_button_key`). A click on a field row focuses it, and one on `Enabled` toggles it (`FormField::at_row`, the Copy dialog's rule); an existing server's dimmed Language, the rule and the help row take none. A press on the frame grabs the form to drag it. The suggestion list takes no click (D11): the tracker records it as covered, so a click on it never reaches a button or row beneath. |
| Exit question | Nothing: Up / Down do nothing there                             | `Yes`, `No`, `Cancel` send `y`, `n`, Esc                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |

The form and the exit question are new dialogs, so each opens centred
(`DialogMouse::reset`), however far the last one was dragged.

### 5.7 Terminal notes, for the parity doc

- Ctrl+Space arrives as NUL on xterm-family terminals, and crossterm reports
  NUL as Ctrl+Space. Some input methods (IBus) take Ctrl+Space.
- On macOS, Ctrl+Space is "Select the previous input source" and Ctrl+Down is
  Mission Control's "Application windows" by default, so neither may reach the
  terminal. Alt+Down (Option+Down) reaches FFM where the terminal sends Option
  as Alt. Otherwise AutoComplete-on typing opens the list by itself.

## 6. Highlighting rules

All colours come from `ui::palette`. Token colours come from
`ui::code::token_style`, so the dialog adds no colour literals. The new
palette names are semantic aliases of colours that already exist:
`STATUS_OK = EXECUTABLE`, `STATUS_PENDING = MARKED` and
`STATUS_FAILED = PROMPT_FAILED`. `COMPLETION_BG` receives the `Color::Cyan`
literal that `ui/completion.rs` hard-codes today, so the command line looks
the same.

| Element                                 | Where                | Style                                                                                                                                    |
| --------------------------------------- | -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Language name                           | list, detail heading | `token_style(Token::Keyword)` (white, bold)                                                                                              |
| `[enabled]`, `[x]`                      | detail heading, list | `STATUS_OK`                                                                                                                              |
| `[disabled]`, `[ ]`                     | detail heading, list | `DISABLED_FG`. A disabled row's command and extensions are dimmed too.                                                                   |
| Extensions `.py .pyi`                   | list, heading        | `token_style(Token::Type)`                                                                                                               |
| An extension in conflict (§8)           | list, heading        | `STATUS_FAILED`                                                                                                                          |
| Modified mark `*`                       | list                 | `MARKED`                                                                                                                                 |
| Problem mark `!` and Problem rows       | list, detail         | `STATUS_FAILED` when it blocks, `STATUS_PENDING` when it warns                                                                           |
| Field labels (`Command:`, `Status:`, …) | detail               | `CODE_NORMAL`                                                                                                                            |
| Command value                           | detail               | `DIRECTORY` (plain white)                                                                                                                |
| Arguments and launch arguments          | detail               | `token_style(Token::Switch)` (Far's switch green)                                                                                        |
| `(none)`, `off`                         | detail               | `DISABLED_FG`                                                                                                                            |
| Status value and availability tag       | detail, list         | By `Availability::tone()` (§11): `Good` → `STATUS_OK`, `Pending` → `STATUS_PENDING`, `Bad` → `STATUS_FAILED`                             |
| Executable path, settings path          | detail, row 1        | `token_style(Token::Str)`                                                                                                                |
| Auto-install JSON                       | detail               | `Highlighter::new(Path::new("recipe.json")).line(json)`, each token through `token_style`. Wrapping splits spans but keeps their styles. |
| Read-only reason, `Error` notice        | row 1, message row   | `STATUS_FAILED`                                                                                                                          |
| `Info` notice, view-mode header         | message row, view    | `CODE_NORMAL`                                                                                                                            |
| Selected list row                       | list                 | Uniform `CURSOR_FG` on `CURSOR_BG`, as in panels. Per-cell colours are not kept on the cursor, because they clash with its ground.       |

The labels (`Command:`, `Arguments:`, `Status:`, `Executable:`,
`Launch arguments:`, `Auto-install:`, `Problem:`) are constants in
`ui/lsp_editor/detail.rs`, the only place that writes them. The status
wording belongs to `Availability::label()`. The existing strings are kept
word for word, so the docs and users' memory stay valid.

The other five `Viewer::report` users are not touched, and
`a_report_body_is_plain_and_its_bar_has_no_file_fields` keeps passing.

## 7. Completion

### 7.1 Sources

All completion logic is pure, in a new `src/complete/servers.rs`. It takes a
`Vocabulary`, never the process environment:

```rust
pub struct Vocabulary<'a> {
    pub catalog: &'a LspSettings,   // LspSettings::catalog() — catalog.json, parsed once
    pub draft: &'a LspSettings,     // what the dialog currently holds
    pub programs: &'a [String],     // app.lsp_probe.programs() (§9.5): never blocks
}
pub enum CompletedField { Command, Arguments, Extensions }
pub fn candidates(field: CompletedField, token: &str, vocabulary: &Vocabulary) -> Vec<Candidate>;
```

| Field      | Token                                                                                        | Pool, in order                                                                                                                                                                                                                                                                                                                                                                                                                                           | Hint                                                                                                                                                  |
| ---------- | -------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| Command    | The text before the caret                                                                    | (1) **Known binaries**: every distinct `command` and npm `binary` in the catalog, and every `command` in the draft (servers the user added), ranked with `matcher::rank` (fuzzy, smartcase). (2) **PATH programs**, only when the token has at least `MIN_PATH_PREFIX = 2` characters: `execs::matching_in(programs, token)` by prefix, kept only when `lsp::launchable_name` accepts the name, ranked with `matcher::rank`, minus names already in (1). | `catalog`, or `catalog · PATH` when the name, or `name.exe` on Windows, is also in `programs` (binary search; the list is sorted). `PATH` for pool 2. |
| Arguments  | The argument at the caret, found by the §9.1 grammar                                         | Every distinct argument in the catalog and the draft, ranked with `matcher::rank`.                                                                                                                                                                                                                                                                                                                                                                       | `catalog` or `in use`                                                                                                                                 |
| Extensions | The word at the caret, split on whitespace or `,`, with one leading `.` removed for matching | Every catalog and draft extension, except those already in this field, ranked with `matcher::rank`, with a leading `.` added.                                                                                                                                                                                                                                                                                                                            | The owning language or languages, for example `typescript`, so a conflict is visible before it is made.                                               |

`lsp::launchable_name(name)` is the rule LSP lookup applies
(`launch::executable`). On Unix it accepts any name, because `execs` already
requires the execute bit. On Windows it accepts only a `.exe`, so the Command
list never offers a `.dll`, a `.mui` or npm's extensionless shim that the
Status row would then report as missing.

The argument pool is only the catalog's arguments (`--stdio`, `start`) plus
those the user already uses. FFM talks to servers over stdin/stdout only, so
the vscode transport switches (`--node-ipc`, `--socket=`, `--pipe=`) are never
offered: accepting one would break the server. There is no hard-coded list,
so the catalog stays the one source of known values.

`execs::matching(prefix)` becomes `matching_in(names, prefix)` plus
`matching(prefix) = matching_in(executables(), prefix)`. That is one prefix
rule, used by the command line and here.

### 7.2 Ranking

The known pool comes first, then the `PATH` pool, as
`complete::candidates::command_candidates` puts built-ins before `PATH`.
Inside each pool, `matcher::rank` orders by (Prefix tier before Fuzzy, points,
shorter, lexicographic). That is total and deterministic.

An empty or one-character token gives the known pool and no `PATH` programs.
One letter matches hundreds of programs, which is a wall, not a choice.
`execs::matching` already returns nothing for an empty prefix for the same
reason. The list is not capped, because the dropdown scrolls
(`completion_placement`).

### 7.3 When the list opens

**The program list.** `Probe::programs()` never blocks. In production it
returns `execs::ready()`: the memoized `PATH` scan once it has finished, and
an empty list until then. `ops::lsp_editor::open` calls
`app.lsp_probe.warm()`, which starts that scan once, on a named background
thread (`ffm-path-scan`), through `execs::warm()`. While the scan runs, the
Command list offers only known binaries. A keystroke never scans `PATH`, and
opening the dialog never waits for it. A thread that cannot be spawned is
logged, and the `PATH` pool stays empty.

**Opening and refreshing** is `ops::lsp_editor::refresh_completion(app, trigger)`
with `enum Trigger { Typed, Requested }`:

- **Any edit** (a character, Backspace or Delete) in a text field that has a
  list open refilters that list, whatever the setting. It keeps the list's
  state (§5.4) and closes it when nothing matches.
- **`Typed`**, with no list open, acts only when
  `app.settings.enabled(SettingsPage::AutoComplete)` is true. It opens a
  **passive** list unless the token is empty, nothing matches, or the only
  match equals the token.
- **`Requested`** is Ctrl+Space, Ctrl+Down or Alt+Down, and ignores the
  setting, as the editor's Ctrl+Space does.
  - No match: the notice reads `No suggestions`.
  - One match: it is accepted outright (the command line's rule).
  - More than one: an **active** list opens on its first row.

The list lives in the form as `completion: Option<Suggestions>`:

```rust
pub struct Suggestions { menu: Menu, focus: ListFocus }
pub enum ListFocus { Field, List }   // Field: passive, nothing highlighted, the form keeps Enter
```

`Menu` cannot be empty, so "the highlight points nowhere" cannot be
represented. A passive list is not "no highlight inside `Menu`". It is a
list whose focus is still on the field.

### 7.4 Accepting

`LineEditor::set_split(head + text, tail)` replaces the token before the caret.

- **Command**: no trailing space.
- **Arguments**: the candidate is quoted with the §9.1 grammar if it needs
  it, and one space is added unless the tail starts with whitespace.
- **Extensions**: `.ext`, with the same trailing-space rule.

### 7.5 Drawing

`ui/completion.rs` gets
`pub(super) fn render_dropdown(f, menu: &Menu, anchor: Rect, side: Side, highlight: Highlight)`,
extracted from `render_completion_menu` together with the private
`completion_items` and `completion_placement`:

- `enum Side { Above, Below }`. The command line keeps `Above`. The form uses
  `Below` and flips to `Above` when the screen has no room.
- `enum Highlight { Selected, None }`. The command line passes `Selected`. The
  form passes `None` for a passive list.

The existing command-line tests pin that nothing changes for the command line.

## 8. Validation

The form checks on OK, and the table checks at F2 and while drawing.

### 8.1 Form (OK)

Each text field is parsed by its newtype (§9.1). On the first error, OK is
refused, focus moves to that field, and the help row shows the message. That
field's value never reaches the draft.

| Case                                                                                        | Result                | Message                                                                                                                  |
| ------------------------------------------------------------------------------------------- | --------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| Command empty or only spaces                                                                | Block                 | `Enter a command.`                                                                                                       |
| Command with a control character or newline                                                 | Block                 | `The command cannot contain control characters.`                                                                         |
| Command starting with `~`                                                                   | Block                 | `No shell runs the server, so ~ is not expanded; write the full path.`                                                   |
| Command not on `PATH` / not runnable                                                        | Accept; §11 shows it  | The Status row of the recomputed availability, for example `not installed; auto-install disabled` in red                 |
| Arguments with an unclosed `"`                                                              | Block                 | `A quote is not closed.`                                                                                                 |
| An argument with a control character                                                        | Block                 | `An argument cannot contain control characters.`                                                                         |
| An extension that is `.` or empty (`.,`)                                                    | Block                 | `An extension cannot be empty.`                                                                                          |
| An extension with `.` inside (`d.ts`), `/`, `\`, whitespace, a comma or a control character | Block                 | `.d.ts can never match: FFM compares the text after the last dot.` / `Extensions cannot contain spaces, commas, / or \.` |
| The same extension twice in one field (`rs RS`)                                             | Block                 | `.rs is listed twice.`                                                                                                   |
| No extensions                                                                               | Accept with a warning | `Problem: no extensions — this server never starts.`                                                                     |
| New server: language invalid or already present                                             | Block                 | `Use a-z, 0-9 and - _ . + (LSP languageId).` / `python already exists; edit it instead.`                                 |

A leading dot is removed, so a `.rs` loaded from a hand-written file (which
never matched) becomes `rs` the first time the server is accepted through the
form. The field shows every stored extension with exactly one dot (`.rs`, never
`..rs`), which is what makes that round trip hold. Whitespace and commas
separate extensions on the line, so a stored extension holding either is
reported as a problem rather than accepted as one value.

**A value the form cannot show.** A one-line field cannot display a control
character. The form therefore does not open on a server whose loaded command,
arguments or extensions contain one. The notice names the field, for example
`lsp.python.args holds a control character; fix settings.json by hand, or
press Del to reset a catalog server.` Other invalid loaded values display
normally and are checked on OK, so the form is how the user fixes them.

### 8.2 Table (F2 and drawing)

`models::lsp_editor::problems(&draft, &baseline) -> Vec<(Problem, Impact)>`,
with `enum Impact { Blocking, Warning }`:

| Problem                                                                                                                                                                                                                       | Blocks F2 when                                                                                                                     | Otherwise                                                           |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| `DuplicateExtension { extension, first, second }`: two **enabled** servers claim one extension (ASCII case-insensitive). At runtime, the one earlier in `BTreeMap` order wins, and the other never starts for that extension. | The draft has it and the baseline does not, compared by `(extension in ASCII lower case, first, second)`. The draft introduced it. | Warning: it is already in the file, whatever else the save changes. |
| `InvalidExtension { language, extension }`, `EmptyCommand { language }`, `NoExtensions { language }`, `ControlCharacter { language, field }`, found in values loaded from the file                                            | Never. Only fields the draft changes are written, and they passed §8.1.                                                            | Warning                                                             |

When F2 is refused:

- the message is `Not saved: .ts is claimed by typescript and deno (both enabled).`;
- the selection moves to the first server involved;
- nothing is written.

## 9. Model types

### 9.1 Values: `src/config/lsp/values.rs`

These are validated newtypes. Each `parse` is the only constructor for user
input, following the `models::wsl::DistributionName::new` pattern.

```rust
/// Whether FFM starts a server. Serialized as JSON true/false, so the file format is unchanged.
#[serde(from = "bool", into = "bool")]
pub enum Enablement { Enabled, Disabled }       // replaces ServerSettings.enabled: bool

pub struct LanguageId(String);   // parse: [a-z0-9][a-z0-9_.+-]{0,63} for a new language;
                                 // from_key: a key already in the table, kept verbatim (renaming is out of scope)
pub struct ServerCommand(String); // trimmed, non-empty, no control chars, no leading '~'
pub struct Arguments(Vec<String>); // parse_line(&str) / to_line(): the D7 grammar; parse_line(to_line(a)) == a
pub struct Extension(String);    // one leading '.' stripped; non-empty; no '.', '/', '\\', whitespace, control
pub struct Extensions(Vec<Extension>); // parse_line: split on whitespace and ','; no ASCII-case duplicates

pub enum ValueError {
    LanguageEmpty, LanguageCharacters, CommandEmpty, CommandControl, CommandTilde,
    QuoteNotClosed, ArgumentControl, ExtensionEmpty, ExtensionDot(String),
    ExtensionSeparator(String), ExtensionTwice(String),
}   // Display: the messages of §8.1, one place
```

`Arguments::to_line` quotes an argument, with its inner quotes doubled, when
it is empty or contains whitespace or `"`. So `--x=a b` becomes
`"--x=a b"`, `a"b` becomes `"a""b"`, and the empty argument becomes `""`.
`parse_line` reads them back. It reads `--x="a b"` as `--x=a b`, and `a""b`
(an empty run between two letters) as `ab`.

`ServerSettings.enabled` becomes `Enablement`. `for_path` then reads
`server.enabled == Enablement::Enabled`, and the catalog, merge and
round-trip tests keep passing, because the JSON is still `true`/`false`. The
draft is **`LspSettings` itself**, not a second struct that repeats
`ServerSettings`' fields (DRY). The newtypes guard the input boundary, which is
the form's OK, and hand their inner values to `ServerSettings`.

`LspSettings::catalog() -> &'static LspSettings` parses `catalog.json` once
(`OnceLock`), and `LspSettings::default()` clones it. `Origin`, the
completion `Vocabulary` and `availability` read the catalog through it.
`LspSettings::deserialize` prefixes each error with `lsp.<language>: `, so a
message names the entry that failed.

### 9.2 The patch: `src/config/lsp/patch.rs`

```rust
pub enum OwnedField { Enabled, Command, Args, Extensions }   // ALL: [Self; 4]; key(): "enabled" …; value_of(&ServerSettings) -> Value
pub enum Change {
    Set    { language: LanguageId, field: OwnedField, value: Value },
    Unset  { language: LanguageId, field: OwnedField },
    Add    { language: LanguageId, server: Value },
    Remove { language: LanguageId },
}
pub struct NotAnObject(pub LanguageId);
pub fn diff(baseline: &LspSettings, draft: &LspSettings) -> Vec<Change>;
pub fn apply(changes: &[Change], lsp: &mut serde_json::Map<String, Value>) -> Result<(), NotAnObject>;
```

`diff` works per server:

- **Catalog language.** For each owned field where draft ≠ baseline:
  - draft = catalog value → `Unset`;
  - otherwise → `Set`.
- **Added language.** `Add` with every field explicit:
  `{enabled, command, args, extensions, "root_markers": [".git"], "install": null}`.
  An omitted field would otherwise inherit rust-analyzer's defaults.
- **Removed added language.** `Remove`.
- **Changed added language.** `Set` for each changed field. It never uses
  `Unset`, because there is no catalog value to fall back to.

`apply`:

- `Set` creates `lsp.<language>` as `{}` when it is absent, then sets the
  field, keeping the other keys in their places.
- `Unset` and `Remove` use `shift_remove` (D6). An `Unset` that empties the
  object removes `lsp.<language>` with `shift_remove`, so the other languages
  keep their order.
- An entry that is not an object gives `NotAnObject`. Such a file cannot
  load, so the `lsp` subtree check (§10.2) stops it first. The error keeps
  `apply` total without a panic.

A test ties `OwnedField::key()` to `ServerSettings`' serde names, so the four
strings cannot drift.

### 9.3 Dialog state: `src/models/lsp_editor.rs` (+ `form.rs`, `problems.rs`)

```rust
pub struct LspEditor {            // private fields, accessors for ui, mutations for ops/popup
    access: Access,
    baseline: LspSettings,        // what the file held at open or at the last save (merged over the catalog)
    baseline_raw: Value,          // the raw `lsp` subtree at open or at the last save (Null when absent)
    draft: LspSettings,
    availability: BTreeMap<LanguageId, Availability>,
    selected: Option<LanguageId>, // None only for an empty table
    mode: Mode,
    notice: Option<Notice>,
}
pub enum Access { Writable { path: PathBuf }, ReadOnly(ReadOnlyReason) }
pub enum ReadOnlyReason { NoSettingsFile, Document(DocumentError) }
pub enum Mode { Browse, View { top: usize }, Form(Box<ServerForm>), ConfirmExit(ExitChoice) }
pub enum Notice { Info(String), Error(String) }
pub enum Origin { Catalog, Added }   // computed from LspSettings::catalog(), not stored — decides Reset vs Remove, Unset vs Set

pub struct ServerForm {
    target: FormTarget,                           // Existing(LanguageId) | New
    language: LineEditor, command: LineEditor, arguments: LineEditor, extensions: LineEditor,
    enabled: Enablement,
    focus: FormField,
    completion: Option<Suggestions>,              // §7.3
    refusal: Option<ValueError>,
}
pub enum FormField { Language, Command, Arguments, Extensions, Enabled, Ok, Cancel }
// ORDER const, allows(FormTarget), is_text(), completed() -> Option<CompletedField>, help() -> &'static str
```

- **Selection is a language**, not an index. The row index is its position
  in the draft, which has a few dozen rows at most. Add selects the new
  language. When a Remove or a save makes the selected language disappear,
  the row now at its old position is selected, clamped to the table. A save
  that brings in servers added or removed by another program therefore
  cannot move the selection to a different server.
- `dirty()` is `draft != baseline`, computed and never stored (the PATH
  editor's rule).
- A server is changed when `draft.0.get(l) != baseline.0.get(l)`. That
  drives the `*` mark.
- Every type here is `Clone`, because `Overlay` derives `Clone`.
  `DocumentError` is `Clone` for that reason (§10.1).
- The anti-patterns listed in the research are avoided: no `bool` parameters
  (movement is `enum Step { Up, Down, PageUp, PageDown, First, Last }`), no
  public fields, no unchecked index, no colour literals and no magic offsets.

### 9.4 Availability: `src/lsp/install/report.rs`

```rust
pub enum Availability {
    Configured(Resolved),            // "found configured executable"
    Cached(Resolved),                // "installed in FFM cache"
    NotInstalled(NotInstalled),
    Replaced(Replacement),           // the command was not found; the recipe's program runs instead
    Failed(InstallError),            // corrupt receipt, I/O, receipt.preview errors (e.g. Java 21 required)
}
pub struct Resolved { pub executable: PathBuf, pub args: Vec<String> }
pub enum NotInstalled { AutoInstallOff, NoCache, AfterTrust }
pub enum Replacement { Cached(Resolved), AfterTrust }
pub enum Tone { Good, Pending, Bad }
pub fn availability(language: &str, server: &ServerSettings, directory: &Path, probe: &Probe) -> Availability;
```

`resolve`, `configured` and the workspace-root lookup stay as they are, with
the probe as their `Environment` and cache. Only the
`(&'static str, ServerSettings)` result becomes a type. The three
`InstallError::Missing("…")` literals that `report.rs` itself creates move to
`NotInstalled::label()`. `InstallError` is unchanged, because the installer
uses it elsewhere.

**When the recipe replaces the command.** `configured()` is tried first, as
today. When it fails, the recipe outcome is `Cached` or
`NotInstalled(AfterTrust)` only when the recipe supplies the configured
command. That is, the language is in the catalog, and the server's `command`
and `install` both equal the catalog entry's. Otherwise the same outcomes are
`Replaced(Replacement::Cached(..))` and `Replaced(Replacement::AfterTrust)`.
`NoCache`, `AutoInstallOff` and `Failed` are unchanged.

A hand-written added server whose own recipe does supply its command also
shows `recipe`. The label is still true, because the recipe's program is what
runs. The amber tone is the price of not guessing what a recipe installs.

`lsp.rs` drops `pub use install::inventory` and gains
`pub(crate) use install::{launchable_name, Availability, NotInstalled, Probe, Replacement, Resolved, Tone};`,
so `ui`, `models` and `ops` can name them while `install` stays private.

### 9.5 The probe: `src/lsp/install/probe.rs`

```rust
/// What the Language servers dialog asks of this machine (D12).
#[derive(Debug, Clone, Default)]
pub struct Probe {
    cache: Option<PathBuf>,        // lsp::install::cache_dir() in production
    path: Option<OsString>,        // PATH, to find a configured command and runtimes
    java_home: Option<PathBuf>,    // JAVA_HOME, for the jdtls runtime
    programs: Programs,            // the Command completion pool
}
#[derive(Debug, Clone, Default)]
pub enum Programs { #[default] Listed(Vec<String>), Process }

impl Probe {
    pub fn from_process() -> Self;                                        // cache_dir(), PATH, JAVA_HOME, Programs::Process
    pub fn new(cache: Option<PathBuf>, path: Option<OsString>) -> Self;  // tests: Listed(execs::scan(path)), no JAVA_HOME
    pub fn programs(&self) -> &[String];                                  // Listed: the list; Process: execs::ready() or empty
    pub fn warm(&self);                                                   // Process: execs::warm(); Listed: nothing
}
impl Environment for Probe { … }   // program(): environment::candidates(names, split_paths(path), java_home)
```

- `program()` uses the same `candidates` function as `System`, so the dialog
  and the runtime find programs by one rule. `run()` returns an error and is
  never reached, because availability only inspects. A test pins that probing
  an install recipe never calls it.
- `complete::execs` gains `ready() -> Option<&'static [String]>` (the
  `OnceLock`'s `get()`, which never blocks) and `warm()` (once, on the
  `ffm-path-scan` thread, calls `executables()`).
- `App` gets `pub(crate) lsp_probe: lsp::Probe`, which `App::from_panels`
  leaves at `Probe::default()`. `bootstrap::load_persisted` installs
  `Probe::from_process()`.
- `test_support::set_lsp_probe(app, cache, bin_dirs)` sets it for the BDD
  crate, which cannot name `lsp`.

## 10. Settings write path

> **Superseded by §21.** This section is the JSON design the dialog was built
> on. The port keeps what F2 writes (§10.3) and drops this module, its three
> owners and its backup in favour of the shared `settings.yaml` document.

### 10.1 One module: `src/config/document.rs`

```rust
pub fn read(path: &Path) -> Result<Option<Vec<u8>>, DocumentError>;     // None: no file
pub fn parse(bytes: &[u8]) -> Result<Map<String, Value>, DocumentError>; // the top level must be an object
pub fn loads(bytes: &[u8]) -> Result<Settings, DocumentError>;          // serde_json::from_slice: errors carry line and column
pub fn update(
    path: &Path,
    backup: Backup,
    edit: impl FnOnce(&mut Map<String, Value>) -> Result<(), DocumentError>,
) -> Result<Saved, DocumentError>;
pub enum Backup { Keep, Skip }
pub struct Saved { pub document: Map<String, Value>, pub settings: Settings }
pub struct JsonError { line: usize, column: usize, message: String }  // from serde_json::Error, which is not Clone
pub enum DocumentError {
    Unreadable(io::ErrorKind), NotRegular, TooLarge(u64), Syntax(JsonError), NotObject,
    DoesNotLoad(JsonError), WouldNotLoad(JsonError), LspChanged, Patch(NotAnObject),
    ChangedWhileSaving, DanglingLink(PathBuf), HardLinked, NotOwned,
    Backup(io::ErrorKind), Write(io::ErrorKind),
}   // Clone. Display: one sentence each, naming settings.json; Write says "settings.json is unchanged"
```

I/O variants carry `io::ErrorKind`, as `InstallError::Io` already does, so
the error can live in the `Clone` overlay.

**`read`**:

- A missing file gives `None`.
- Otherwise the file must be a regular file of at most 1 MiB. This guard
  moves here from `ops::menu::settings::read_json`, which then calls it. A
  FIFO is refused before it is opened, so it cannot block the UI thread.
- There is no fallback to defaults.

**`loads`** deserializes from the bytes, not from a `Value`, so every error
carries its line and column. Together with the `lsp.<language>: ` prefix
(§9.1), a message reads
`settings.json does not load: lsp.python: invalid type: string "x", expected a sequence at line 7 column 20`.

**`update`**, in order. A failure at any step leaves the user's file
untouched. Steps 1 to 7 never write it, and step 8 replaces it only in its
final rename.

1. `target = file_write::real_target(path)?`. A symlink resolves to the file
   it names. A dangling symlink is `DanglingLink(read_link(path))`: the rename
   would otherwise replace the link with a regular file. A path that does not
   exist and is not a link is used as given. This is the editor's
   `resolve_save_target`, moved to `file_write` and shared. The editor then
   refuses a dangling link too, instead of replacing it.
2. For an existing target: `file_write::ensure_writable(&target)`, then, on
   Unix, `HardLinked` when it has other names and `NotOwned` when another
   user owns it. `has_other_names` and `owned_by_us` move from `ops/editor.rs`
   to `file_write` and are shared. An atomic write would detach the other
   names or take the file over, and an in-place write is not atomic, so the
   save is refused.
3. `before = read(target)?`. When present: `loads(before)?`, which gives
   `DoesNotLoad` with the file's own positions, then `document = parse(before)?`.
   When absent: `document = {}`.
4. `edit(&mut document)?`. This is the caller's owned-keys change and its own
   checks.
5. `text = to_string_pretty(document) + "\n"`, then `settings = loads(text)`.
   A failure is `WouldNotLoad`: a writer bug, pinned by tests, and nothing is
   written.
6. Re-read `target`. If the bytes differ from `before`, return
   `ChangedWhileSaving` ("press F2 again"). Nothing is lost, because the next
   F2 recomputes from the draft.
7. With `Backup::Keep` and a `before`, write `<path>.bak`, which is
   `settings.json.bak` beside `path` in the config directory, not beside the
   resolved target. It is written atomically with those bytes. A failure is
   `Backup`, and nothing else is written.
8. `create_dir_all(parent)`, then `file_write::temp_beside(target)` and
   `persist_over(temp, target, text)`. That is a temp file in the same
   directory, the target's permissions copied (a new file stays 0600),
   `sync_all`, then rename.
9. On Unix, `file_write::sync_directory(parent)` makes the rename durable. It
   is best effort: errors are ignored, because the rename has already
   happened.

The race left between steps 6 and 8 is milliseconds wide. A lock file would
cost more than it saves. D4's `lsp` subtree check already covers the
realistic case of two FFM instances editing servers.

### 10.2 The three owners

| Writer                                                                            | Backup | Edit closure                                                                                                                                                                                                                                                                                                                                                  |
| --------------------------------------------------------------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **LSP dialog** (F2, or Yes in the exit question)                                  | `Keep` | If `document.get("lsp")` (`Null` when absent) differs from `baseline_raw`, return `LspChanged`. Otherwise `patch::apply(changes, lsp)`, creating `lsp` as `{}` when absent. `NotAnObject` becomes `Patch`.                                                                                                                                                    |
| **Save setup** (`ops::menu::settings::save`: Shift+F9, F9 → Options → Save setup) | `Skip` | For each page in `app.settings.values()`, set `document[page_name] = String(value)`, where `page_name` is the serde name. A page it holds no value for is left as it is, and it never removes a key. The page keys never affect `config::Settings`, because serde ignores them there.                                                                         |
| **`config::Settings::save`** (exit auto-save)                                     | `Skip` | For each top-level key of `to_value(self)` except `lsp`, `merge(document[key], value)`. It sets only the keys the struct defines, at every object level, and replaces a non-object value (arrays included) whole. A key that a newer FFM or the user added inside a page survives. `lsp` belongs to the dialog, which keeps it in sync with `app.config.lsp`. |

`update` itself runs `loads(before)` for every writer. For
`config::Settings::save` that is the old guard. If the file does not load,
`app.config` came from the defaults, and writing it would overwrite the
user's values. For Save setup, it means a file that startup rejects is never
rewritten. The message names the line.

`ops::savesetup` is deleted with its tests, and so are
`ToolCommand::SaveSetup`, its fallback row in `action/function_keys.rs` and
the `None` arm of `application::dispatch::actions::apply_tool`. The test at
`tests_command_dialogs.rs:176-191` is replaced by one that pins Shift+F9 to
`MenuCommand::SaveSetup`.

`ops::menu::settings::load(app, path)` first sets
`app.settings.persistence = File(path)` (D5). It then reads through
`document::read` and `parse`, keeps the keys that deserialize as
`SettingsPage` and skips every other key. A page key whose value is not a
string is still an error, as today. On any error the pages stay at their
defaults, and only the status reports it. `load_default` uses
`config::Settings::path()`, and `load_from_environment` is deleted.
`config::Settings::load` reads through `document::read` and `loads`. It still
falls back to the defaults on error, but now with the FIFO and size guard.

### 10.3 Dialog open and save

**`ops::lsp_editor::open(app)`**:

1. `path = app.settings.persistence.file()`.
2. With no path: `Access::ReadOnly(ReadOnlyReason::NoSettingsFile)`, and the
   table shown is `app.config.lsp`, the servers FFM would start.
3. Otherwise `read(path)`, then `loads` and `parse` of the bytes. A missing
   file is the catalog and an empty object.
   - On `Err(e)`: `ReadOnly(ReadOnlyReason::Document(e))`, and the table
     shown is `app.config.lsp`.
   - On success: `Writable { path }`, with `baseline = draft = loaded.lsp` and
     `baseline_raw = document["lsp"]` (`Null` when absent).
   - The dialog shows the file, because the file is what F2 writes. Until the
     first save, the editor keeps what was loaded at startup.
4. Resolve `availability` for every server, with the active panel as the
   directory and `app.lsp_probe`.
5. `app.lsp_probe.warm()`, then open `Mode::Browse` with the first row
   selected.

Opening never installs or starts anything, and never waits for the `PATH`
scan.

**`ops::lsp_editor::save(app)`** is `save_with(app, document::update)`, the
PATH editor's seam:

1. `Access::ReadOnly` gives an `Error` notice with the reason.
2. Blocking problems (§8.2) give an `Error` notice and select the server.
3. `changes = patch::diff(..)`. An empty result gives `Info("Nothing to save")`.
4. `written = write(path, Backup::Keep, closure)`.
   - On `Err(e)`: `Error(e.to_string())`. The draft and the baseline are
     unchanged, and the dialog stays dirty.
   - On `Ok(Saved { document, settings })`:
     - `app.config.lsp = settings.lsp.clone()`, so the next editor start uses
       the file;
     - `baseline = draft = settings.lsp` (this also brings in unrelated
       external edits);
     - `baseline_raw = document["lsp"]`;
     - the selection is found again by language (§9.3);
     - availability is resolved again;
     - `Info("Saved to <path>. Changes apply the next time a file is opened in the editor.")`.
       No server runs while the dialog is open (§2), so there is nothing to
       stop or restart.

### 10.4 What happens on error

> **Superseded by §21** where it names JSON positions, `settings.json.bak`,
> the re-read before the rename or the "is unchanged" wording.

| Situation                                                                                      | Result                                                                                                                                                                                                                       |
| ---------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| File missing                                                                                   | Editable. The first F2 creates the directory and the file.                                                                                                                                                                   |
| Not JSON, not an object, not a regular file, over 1 MiB, or not loadable as `config::Settings` | The dialog opens read-only and names the reason with its position, for example `settings.json is not valid JSON: line 4 column 2 …` or `settings.json does not load: lsp.python: … at line 7 column 20`. Nothing is written. |
| The Options pages failed to load at startup                                                    | No effect on the dialog. It knows the file (D5) and checks it itself.                                                                                                                                                        |
| No home or `APPDATA` (a `SessionOnly` app)                                                     | Read-only: `no settings file for this session`.                                                                                                                                                                              |
| `settings.json` is a symlink to a missing file                                                 | F2 → `DanglingLink`, naming the missing target. The link is untouched.                                                                                                                                                       |
| Other hard links, another owner (Unix), or not writable                                        | F2 is refused, and the message says which. File untouched.                                                                                                                                                                   |
| Another program changed `lsp` while the dialog was open                                        | F2 → `lsp in settings.json changed outside this dialog. Close without saving and open it again to see the change.` Draft kept.                                                                                               |
| The file changed into something unloadable while the dialog was open                           | F2 → `DoesNotLoad`. Draft kept. File untouched.                                                                                                                                                                              |
| The file changed between read and rename                                                       | `ChangedWhileSaving`. Draft kept. F2 again.                                                                                                                                                                                  |
| Backup cannot be written                                                                       | `Backup` error. Nothing written.                                                                                                                                                                                             |
| Disk full, permission denied, read-only filesystem                                             | `Write` error. The temp file is dropped, and the original is untouched because the rename never happened.                                                                                                                    |
| Crash during the write                                                                         | The rename is atomic: the old file or the new one, never half.                                                                                                                                                               |

After every successful dialog save, the previous file is in
`settings.json.bak` beside `settings.json`. Only dialog saves write it. The
docs say how to restore it.

## 11. Install status and auto-install

- **Status row**: `Availability::label()`, in the colour of
  `Availability::tone()`.

  | Availability                   | Label                                                                         | Tone    | List tag  |
  | ------------------------------ | ----------------------------------------------------------------------------- | ------- | --------- |
  | `Configured`                   | `found configured executable`                                                 | Good    | `ready`   |
  | `Cached`                       | `installed in FFM cache`                                                      | Good    | `ready`   |
  | `NotInstalled(AfterTrust)`     | `not installed; installs after workspace trust`                               | Pending | `auto`    |
  | `Replaced(Cached)`             | `command not found; FFM runs the auto-installed server instead`               | Pending | `recipe`  |
  | `Replaced(AfterTrust)`         | `command not found; FFM will install and run the auto-install server instead` | Pending | `recipe`  |
  | `NotInstalled(AutoInstallOff)` | `not installed; auto-install disabled`                                        | Bad     | `missing` |
  | `NotInstalled(NoCache)`        | `no installation cache configured`                                            | Bad     | `missing` |
  | `Failed(e)`                    | `e.label()`                                                                   | Bad     | `error`   |

  The first three, `NotInstalled(AutoInstallOff)` and `NotInstalled(NoCache)`
  keep their old wording.
- **Executable** and **Launch arguments** rows appear for `Configured`,
  `Cached` and `Replaced(Cached)`. They show what FFM will actually run, for
  example the cached `pylsp` under a `recipe` tag, or the jdtls `-data`
  directory of the active panel's project.
- **A changed command that shows `recipe`** is not being run. The docs say
  how to make it stick: install it on `PATH`, enter its full path, or set
  `"install": null` for that language by hand (D11).
- **Auto-install row**: the compact recipe JSON, highlighted (§6), or `off`
  (dimmed) when `install` is `None`. It is shown, not edited (D11).
- **Recomputing** happens at open, after a form's OK (that server only),
  after Space and Del (that server), after a save (all servers) and on F5
  (all servers). It never happens per keystroke. The availability always
  describes the **draft**, which is what F2 will write, not a running
  process. A check looks up one path per `PATH` directory per server, as the
  report does today. It never lists a directory.
- **View-mode header**: `Availability is what FFM would start; it is not a
  process list.`
- A disabled server's availability is still shown, so the user can see
  whether enabling it would work.

## 12. Cancel and recovery

| Level  | Mechanism                                                                                                                                           |
| ------ | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| Field  | Esc in the form cancels the whole form. The draft is untouched, because values reach it only on OK. The suggestion list's Esc closes only the list. |
| Server | Del resets a catalog server's four owned fields, or removes an added one, in the draft only.                                                        |
| Dialog | Esc or F10 with `dirty()` asks Save / Discard / Cancel (§4.4). Discard drops the draft and writes nothing. A clean dialog closes at once.           |
| File   | Superseded (§21): no backup is kept. `settings.json.bak` kept the version before the last dialog save.                                              |

There is no undo (D11). Nothing reaches the file before F2, and Discard
recovers from any mistake made since the last save.

## 13. Tests

All tests go in sibling files, with no network. `PATH` data comes from
`execs::scan` over a `TempDir`. Availability comes from
`Probe::new(Some(temp cache), Some(fake PATH))`, so no test reads the real
LSP cache, `PATH` or `JAVA_HOME`. Settings files come from
`Settings::with_file(temp/config/settings.json)`. Write-failure tests use a
blocked parent (a file where the directory should be), a directory in place
of the file, or the injected writer. Chmod tests are skipped under root
(`platform::is_root()`). JSON is compared as parsed `Value`s, except in the
tests that pin key order.

### 13.1 Unit tests

| File                                       | Covers                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| ------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `config/lsp/values/tests.rs`               | Table-driven `parse` for each newtype, one row per §8.1 case. `Arguments` round trip over `--stdio`, `a b`, `""`, `a"b`, `--x=a b`, `C:\tools\x`, `ü`, `--x=` and many spaces. `--x="a b"` reads as `--x=a b`, and `a""b` as `ab`. A control character in any argument is `ArgumentControl`. `Extensions` rejects `rs RS`. `Enablement` serde `true`/`false` both ways. `LanguageId::from_key` keeps a key verbatim.                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `config/lsp/tests.rs`                      | Catalog and merge tests with `Enablement`. `for_path` skips `Disabled`. `LspSettings::catalog()` equals `default()`. A deserialize error names `lsp.<language>`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| `config/lsp/patch/tests.rs`                | Disabling a catalog server gives `Set enabled=false`, and re-enabling it gives `Unset`, after which the object is gone. An untouched server has no change. An added server is written in full (`root_markers [".git"]`, `install null`). Reset keeps `root_markers`, `install: null` and unknown fields. Remove works. An `Unset` keeps the order of the sibling keys, and a Remove keeps the order of the other languages (`shift_remove`). A non-object entry gives `NotAnObject`. `OwnedField::key()` equals the serde keys.                                                                                                                                                                                                                                                                                                                                                        |
| `config/document/tests.rs`                 | `read`: missing, `[]`, `{broken` (the Display has a line and a column), a directory, over 1 MiB, a FIFO (Unix, `make_fifo`, bounded by `run_with_timeout`). `update` keeps unknown keys, `SettingsPage` keys and nested unknowns. `.bak` is written only with `Backup::Keep`, beside the link, and holds the old bytes. A symlink stays a link and its target is updated. A dangling symlink is refused and stays a link. A hard-linked file and a 0444 file are refused (Unix, not root). A closure error leaves the bytes and `.bak` unchanged. A blocked parent fails without creating anything. Existing permissions are kept (Unix). A new file is 0600 (Unix). The output ends with `\n`. `ChangedWhileSaving` is triggered by a closure that rewrites the file. `DoesNotLoad` comes from a file whose `panel` is `5`, and `WouldNotLoad` from a closure that writes `"lsp": 5`. |
| `file_write/tests.rs`                      | `real_target`: a link, a chain of links, a dangling link, a missing plain path. `has_other_names` and `owned_by_us` moved unchanged.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `ops/editor` tests                         | Existing save tests unchanged. New: saving through a dangling link is refused and the link is kept.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `config/tests.rs`                          | `Settings::save` keeps foreign keys, keeps `{"panel": {"future": 1}}`, leaves `lsp` untouched, and refuses on a file that does not load. `Settings::load` does not block on a FIFO.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `ops/menu/settings/tests.rs`               | `load` ignores `lsp` and lowercase pages and raises no error status. `load` sets the file even when it fails, on `{broken` and on `"Editor": "20"`. `save` writes only `values()`, keeps `lsp`, unknown keys and the page keys it holds no value for. `load_default` follows `config::Settings::path()` (`EnvVarGuard`). Its `CONFIG_HOME` constant is the second copy after `config/tests.rs`, left with a TODO under the Rule of Three. The existing precedence test is deleted with `load_from_environment`.                                                                                                                                                                                                                                                                                                                                                                        |
| `action` tests, `tests_command_dialogs.rs` | Shift+F9 resolves to `MenuCommand::SaveSetup`. The dead `Tool` route is gone.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `lsp/install/report/tests.rs`              | `availability` with the `Packages` fake and a probe gives each `Availability` variant. A catalog python with a changed command and a cache gives `Replaced(Cached)`, without a cache `Replaced(AfterTrust)`, and with its catalog command `Cached`. Labels match the old strings where they are kept. The tone mapping is checked.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `lsp/install/probe/tests.rs`               | `Probe::new` finds a program only on its own `PATH`. `Probe::default()` finds nothing. A `Listed` probe returns its scan. Probing a recipe never calls `run`. `launchable_name` keeps only `.exe` on Windows (`cfg(windows)`).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `complete/servers/tests.rs`                | Command: catalog before `PATH`, deduplicated, with the merged `catalog · PATH` hint. An empty or one-character token gives the catalog only. Fuzzy `ra` finds `rust-analyzer`. Arguments offer only the catalog and draft values, never `--socket=`. Extensions: owner hint, current-field exclusion, leading dot. Smartcase.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `complete/execs/tests.rs`                  | `matching_in` gives the same answers as `matching`. After `warm()`, `ready()` becomes `Some` within a bounded wait.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `models/lsp_editor/tests.rs`               | Selection bounds (`Step`). The selection follows its language across a save that adds or removes an earlier server, and is clamped when its language disappears. `dirty`. The `*` mark. `problems`: a duplicate among enabled servers only. It blocks only when the draft introduced it: a conflict already in the baseline, plus an unrelated edit to one of its servers, is a warning. Form tab order skips Language for an existing server (`cycle::step`). A refused OK focuses the field. The form refuses to open on a loaded control character. A passive list becomes active on Down.                                                                                                                                                                                                                                                                                          |
| `ops/lsp_editor/tests.rs`                  | `open` with no file (catalog, writable), `{broken` (read-only with line and column), an `lsp` that does not load, and `SessionOnly`. After `ops::menu::settings::load` fails, `{broken` opens read-only with the syntax reason, and `"Editor": "20"` opens writable. Availability comes from `app.lsp_probe`: a program on the fake `PATH` is found, and an empty probe finds nothing. `save_with` with a failing writer keeps the draft, the baseline and dirty. A successful save updates `app.config.lsp`, the baseline and the `.bak`. A raw `lsp` changed on disk gives `LspChanged` and writes nothing. A blocking problem refuses and writes nothing. `accept_form` rechecks one server. The `lsp` command and the Commands menu row open `Overlay::LspEditor`.                                                                                                                 |
| `popup/lsp_editor/tests.rs`                | Every row of §5: an Esc on a clean dialog closes it; an Esc on a dirty one goes to `ConfirmExit`; Y/N/C; Space; F2 to a temp file; Ins, then OK; Del as reset and as remove; F3 and its scrolling; F5; the form's Tab, Shift+Tab, Enter, Esc and Space on Enabled; Ctrl+Space, Ctrl+Down and Alt+Down. Both list states: Enter on a passive list presses OK, and Down then Enter accepts. Typing refilters an open list with AutoComplete off. The auto-open follows the AutoComplete setting. A read-only dialog repeats its reason.                                                                                                                                                                                                                                                                                                                                                  |
| `application/dispatch/dialog_mouse` tests  | A click on `{ OK }`, `[ Cancel ]`, `Yes`, `No` and `Cancel` sends the §5.6 key. The wheel moves the list selection.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |

### 13.2 Screen tests

These use `WritableTestBackend`, `crate::ui::ui` and `buffer_text` for text,
and `buf[(x, y)].fg` against `palette::*` for colours.

| File                             | Covers                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| -------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ui/lsp_editor/tests.rs`         | The §4.1 rows at 80×24: the list columns, `*`, `!`, the detail labels, the legend and the key bar labels per mode and per Origin (`8Reset` / `8Remove`). Colours: the language is bold `CODE_KEYWORD`, `[enabled]` is `STATUS_OK`, `[disabled]` is `DISABLED_FG`, each status is its tone (the `recipe` tag is `STATUS_PENDING`), a JSON string cell is `CODE_STRING`, a conflicting extension is `STATUS_FAILED`, and the selected row is uniform `CURSOR_*`. The form's Language field is dimmed for an existing server. The form's caret in a scrolled field with wide characters. The dropdown sits below the field and flips above near the bottom. A passive list has no highlighted row. The view mode position text. 60×16 and 40×10 draw without a panic or an overflow, and the detail pane is hidden. |
| `ui/completion` (existing tests) | The command-line dropdown is unchanged after the extraction.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `ui/viewer/tests.rs`             | Unchanged: reports stay plain.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `ui/tests.rs` (key bar)          | `render_command_bar` output is unchanged after the labels become a parameter.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |

### 13.3 TUI BDD: `tests/tui_features/lsp.feature`, rewritten

New steps go in `tests/tui_bdd/mod.rs`.
`Session::language_servers(left, right, settings, size)` builds on
`start_app`. It sets
`app.settings = Settings::with_file(workspace/config/settings.json)` and
calls `test_support::set_lsp_probe(&mut app, workspace/lsp-cache, [workspace/bin])`
with an empty `bin`. No scenario reads the real home, `PATH` or LSP cache.
Every catalog server is therefore `auto` or `missing` on every machine.

- `the settings file holds the fixture server`: writes
  `{"System": "5000", "unknown": {"keep": 1}, "lsp": {"fixture": {"command": "<abs fixture-server>", "args": ["--stdio"], "extensions": ["fx"], "root_markers": [".git"], "install": null}}}`.
  `System` differs from its default (`100000`), so a rewrite to defaults is
  visible.
- `the settings file contains {string}`: writes raw text.
- `I open the language servers workspace`: starts the session.
- `the settings file value {string} is {string}`: a dot path compared as
  parsed JSON.
- `the settings file is unchanged`: the bytes equal what the Given step wrote.

`parse_key` already knows Space, Tab, F-keys, `Ctrl+` and `Shift+`. Add
`Insert` only if a scenario uses it; these scenarios use F7 and F8.

The list is sorted: css, fixture, html, java, javascript, javascriptreact,
json, python, rust, …. The dialog opens on css.

| Scenario                            | Steps (abridged)                                                                                                                                                                                                                                                                                                                                                  |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The Commands menu opens the dialog  | `F9 Right Right Down Enter` → the screen contains `Language servers`, the settings path, `fixture` and `2Save`. Press `Down` (css → fixture) → the screen contains `found configured executable`.                                                                                                                                                                 |
| The `lsp` command opens it          | Type `lsp`, press Enter → the screen contains `Language servers`                                                                                                                                                                                                                                                                                                  |
| Toggle and save keep foreign keys   | Press `Down` (css → fixture), then `Space F2` → `lsp.fixture.enabled` is `false`, `unknown.keep` is `1`, `System` is `"5000"`                                                                                                                                                                                                                                     |
| Completing a command                | Press `Down` ×7 (to python), `F4`, Backspace ×5, type `rust-an` → the screen contains `rust-analyzer` and `catalog` (a passive list). Press `Down Enter` (activate, accept), `Enter` (OK) → the screen contains `recipe`, because rust-analyzer is not on the fake `PATH` and python's recipe would run instead. `F2` → `lsp.python.command` is `"rust-analyzer"` |
| A duplicate extension blocks saving | On fixture, press `F4`, Tab ×2, type ` py` (a passive list opens on `.py`), press `Enter` (OK, because a passive list does not take Enter), `F2` → the screen contains `claimed by`, and the file is unchanged                                                                                                                                                    |
| Discarding                          | Press `Space Esc` → the screen contains `Save?`. Press `n` → the file is unchanged and the dialog is closed                                                                                                                                                                                                                                                       |
| An unreadable file is read-only     | The settings file contains `{broken`. Open the dialog → the screen contains `read only` and `line 1`. Press `F2` → the file is unchanged                                                                                                                                                                                                                          |
| Shift+F9 keeps the LSP edit         | Press `Down Space F2 Esc Shift+F9` → `lsp.fixture.enabled` is still `false` and `System` is still `"5000"`                                                                                                                                                                                                                                                        |

BDD runs on Windows in CI too. The fixture server keeps the existing
`write_executable` approach of `Session::inventory` (which it replaces), and
no assertion assumes `/`.

### 13.4 Repository gates

- `tests/public_api_coverage_test.rs`: every new `pub` or `pub(crate)` fn is
  named in a test. Delete entries from `UNEXERCISED` if a new test starts
  calling one. `lsp::inventory` is removed. It is not in `UNEXERCISED`, so the
  list does not change for it.
- `tests/file_line_limit_test.rs`: each new file stays under 300 lines where
  possible and under 1000 always. `ui/overlays.rs` (845) gets one arm and
  nothing more. `ui/dialog.rs` (833) is not touched.
- `tests/module_boundaries`: `ui/lsp_editor*` uses only `models`, `config`,
  `lsp::{Availability, Tone, …}` (the §9.4 re-exports), `highlight` and `ui`,
  never `std::fs`, `ops` or `popup`. `models/lsp_editor*` never uses `ops`,
  `ui` or `popup`. File and `PATH` I/O stay in `ops`, `config`, `lsp` and
  `complete::execs`.
- Clippy: functions of at most 30 lines, nesting of at most 4, no wildcard
  imports. The popup key map is split into one function per mode, as in
  `popup/path_env.rs`.
- The ignored capture test becomes `capture_lsp_editor_review_screen`, which
  draws the new dialog with synthetic paths. `docs/images/lsp-inventory.png`
  is regenerated from it.

Gate commands:

- `./ffm.py lint`
- `cargo test`
- `cargo test --features test-helpers --test tui_bdd`

Run them with `PATH` prefixed by the guest bin directory that provides
`python` for the LSP tests, and at cargo's default job count.

## 14. Docs to update

| File                                                                     | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `docs/knowledge-base/09-far-manager-compatibility/hotkeys-and-parity.md` | Add a **Language servers dialog** section: the §5 tables with a Far-precedent column, the §5.6 mouse table (wheel as Up/Down; list rows and the key bar not clickable), and the §5.7 terminal notes, including the macOS defaults and Alt+Down. Under Overlay / Dialog Keys, note that Shift+F9 now keeps keys it does not own and refuses a file that does not load.                                                                                                                                        |
| `docs/editor-lsp.md`                                                     | Rewrite the Configuration section: the dialog; its keys (linking the parity doc); what F2 writes (only changed fields, partial overrides stay partial); when changes apply (the next time a file is opened in the editor, because no server runs while the dialog is open); the `recipe` status and how to make a changed command stick; `settings.json.bak` from dialog saves and how to restore it; the read-only and refused cases. Remove "Restart FFM after changing settings". Replace the screenshot. |
| `docs/knowledge-base/06-ui-and-interaction/overlays-and-modals.md`       | Add `LspEditor` to the overlay list and the implementation-location checklist. The report paragraph no longer names the LSP inventory.                                                                                                                                                                                                                                                                                                                                                                       |
| `docs/knowledge-base/07-core-features/main-menu.md`                      | Correct the settings path precedence to `config::Settings::path()`. State that Save setup writes only its own keys, and that the Commands row opens the editor. Give the AutoComplete page's new caption.                                                                                                                                                                                                                                                                                                    |
| `README.md` (line 23)                                                    | "Inspect and edit servers through F9 → Commands → Language servers (LSP)."                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `docs/images/lsp-inventory.png`                                          | Regenerated (§13.4).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |

## 15. Ordered task list

Each task is one reviewable commit that leaves every gate green.

1. **T1: Shared settings document.**
   - Add `serde_json` `preserve_order` (D6), after the `rg` audit.
   - Create `config/document.rs` (`read`, `parse`, `loads`, `update`,
     `Backup`, `Saved`, `JsonError`, `DocumentError`), with the 1 MiB
     regular-file guard moved from `ops/menu/settings.rs`.
   - Move `real_target` (now refusing a dangling link), `has_other_names` and
     `owned_by_us` from `ops/editor.rs` to `file_write`, and add
     `file_write::sync_directory`.
   - `LspSettings::deserialize` prefixes `lsp.<language>: `.
   - `config::Settings::load` goes through `read` and `loads`.
   - Tests: `config/document/tests.rs`, `config/tests.rs`,
     `file_write/tests.rs`, `ops/editor` tests (plus the dangling link).
2. **T2: One owner per key.**
   - `ops::menu::settings::{load, save}` go through the document. `load` sets
     the file first, and `save` writes only `values()`.
   - `load_default` uses `config::Settings::path()`, and
     `load_from_environment` is deleted.
   - `config::Settings::save` merges key by key, skips `lsp` and goes
     through `update`.
   - Delete `ops/savesetup.rs` and its tests, `ToolCommand::SaveSetup`, its
     row in `action/function_keys.rs` and the `apply_tool` arm in
     `application/dispatch/actions.rs`. Rewrite the Shift+F9 test in
     `tests_command_dialogs.rs`.
   - The AutoComplete caption in `models/settings.rs`.
   - Tests: `ops/menu/settings/tests.rs`, `config/tests.rs`, the action
     tests. Docs: `main-menu.md`.
3. **T3: LSP values.**
   - `Enablement` goes into `ServerSettings`.
   - `config/lsp/values.rs` gets the newtypes and `ValueError`.
   - `LspSettings::catalog()`.
   - Tests: `config/lsp/values/tests.rs`, `config/lsp/tests.rs`,
     `application/lsp_jobs/tests_session.rs` and `report/tests.rs` (updated
     for `Enablement`).
4. **T4: LSP patch.**
   - `config/lsp/patch.rs` (`OwnedField`, `Change`, `diff`, `apply`,
     `NotAnObject`).
   - Tests: `config/lsp/patch/tests.rs`.
5. **T5: Probe and typed availability.**
   - `lsp/install/probe.rs` (`Probe`, `Programs`, its `Environment`), and
     `launchable_name` next to `launch::executable`.
   - `complete/execs.rs`: `ready` and `warm`.
   - `report.rs` gets `Availability`, `Resolved`, `NotInstalled`,
     `Replacement`, `Tone` and `availability()`. Delete `inventory`, `report`
     and `arguments`.
   - `lsp/install.rs`: drop `pub use report::inventory`, add `mod probe`
     and the `pub(super)` re-exports. `lsp.rs`: drop
     `pub use install::inventory`, add the §9.4 `pub(crate) use`.
   - `models/app.rs`: the `lsp_probe` field. `bootstrap.rs`:
     `Probe::from_process()`. `test_support`: `set_lsp_probe`.
   - Tests: `lsp/install/report/tests.rs`, `lsp/install/probe/tests.rs`,
     `complete/execs/tests.rs`.
6. **T6: Completion sources.**
   - `execs::matching_in`.
   - `complete/servers.rs` (`Vocabulary`, `CompletedField`, `candidates`,
     `MIN_PATH_PREFIX`).
   - Tests: `complete/servers/tests.rs`, `complete/execs/tests.rs`.
7. **T7: Dialog state.**
   - `models/lsp_editor.rs` with `form.rs` and `problems.rs`, including
     `Suggestions`, `ListFocus` and `ReadOnlyReason`.
   - Add the `Overlay::LspEditor(Box<LspEditor>)` variant and its
     `application/timing.rs` name.
   - Tests: `models/lsp_editor/tests.rs`.
8. **T8: Effects.**
   - `ops/lsp_editor.rs` replaces `ops/lsp_status.rs`: `open`, `recheck`,
     `accept_form`, `toggle`, `remove_or_reset`, `save`/`save_with`,
     `refresh_completion`.
   - Rewire `ops::menu::execute_tools` (`MenuCommand::LanguageServers`) and
     `ops::dispatch` (`Command::Lsp`).
   - Tests: `ops/lsp_editor/tests.rs`, which replaces `ops/lsp_status/tests.rs`.
9. **T9: Keys and mouse.**
   - `popup/lsp_editor.rs`, with one function per mode.
   - Add the arm in `popup::process_popup_input`.
   - Add the `Overlay::LspEditor` arm in
     `application/dispatch/dialog_mouse/button_keys.rs`.
   - Tests: `popup/lsp_editor/tests.rs`, the dialog-mouse tests.
10. **T10: Rendering and highlighting.**
    - Palette aliases (`STATUS_*`, `COMPLETION_BG`).
    - `ui/fkey_bar.rs`: the labels become a parameter (`render_key_bar`),
      and `render_command_bar` passes `FKEY_LABELS`.
    - `ui/completion.rs`: `render_dropdown` extracted, with `Side` and
      `Highlight`.
    - `ui/lsp_editor.rs` with `detail.rs` (`detail_lines`, the labels) and
      `form.rs`, plus the `ui/overlays.rs` arm. `ui/dialog.rs` is not changed,
      because `Row::Field` already draws `↓`.
    - Tests: `ui/lsp_editor/tests.rs` and the existing `ui/tests.rs` and
      completion tests.
11. **T11: TUI BDD.**
    - `Session::language_servers`, the §13.3 steps, and a rewritten
      `lsp.feature`.
    - `cargo test --features test-helpers --test tui_bdd`.
12. **T12: Docs and screenshot.**
    - The §14 files.
    - `capture_lsp_editor_review_screen` and the regenerated PNG.
13. **T13: Final gate.**
    - `./ffm.py lint`, `cargo test`, and the BDD run.
    - Public-API coverage and the line limits.
    - A manual run in the TUI: open, edit, save, then Shift+F9, a restart,
      and check that no startup "Settings:" error appears. Then break
      `settings.json` by hand, start FFM, and open the dialog: it opens
      read-only and names the line.

## 16. Out of scope

- Editing `root_markers` and `install` (D11), including an Auto-install
  checkbox. If `recipe` turns out to be common, the follow-up is a fifth
  owned field whose two values are "the catalog recipe" (`Unset`) and `null`.
- Undo in the dialog (D11).
- Clicks on list rows and on the dialog's key bar (D11). FFM hit-tests only
  the panels' key bar today.
- Restarting servers from the dialog. None runs while it is open (§2).
- Refreshing `execs::executables()` after `PATH` changes inside a session. It
  is memoized for the life of the process, and the docs say so.
- Unifying `models::settings::Settings` and `config::Settings` into one
  Options model, and deleting the unread `config::Settings.auto_complete`. D3
  makes the two models coexist safely in one file. Merging them is its own
  task.

## 17. Review

An adversarial review of the first draft (commit `c3dc3f9`) checked it
against the code at `d78f47c` and raised 18 findings. Each was checked
against the code again before it was accepted or rejected.

| #   | Severity | Finding                                                                                                                                                                     | Verdict          | What changed, or why not                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| --- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | major    | The dialog's writability depended on the Options loader: any failed page load left `SessionOnly`, so the dialog said "no settings file" even for a good `lsp`.              | Accepted         | D5: `load` sets the file before parsing. Save setup writes only `values()` (D3, §10.2), so a failed load cannot make it write defaults over the user's pages. An ops test covers `{broken` and `"Editor": "20"` after a failed load. The reviewer's first fix was chosen over a second `settings_file` field, which would be two copies of one fact.                                                                                                                                             |
| 2   | major    | A changed command that is not found is silently replaced by the auto-install recipe, and the dialog showed it as `ready`.                                                   | Accepted in part | `Availability::Replaced` with the `recipe` tag in Pending (D9, §9.4, §11), and a BDD step. Rejected: an Auto-install checkbox in the form. The user asked for command, arguments, extensions and enabled. The status now tells the truth and the docs give three remedies, including `"install": null` by hand. The checkbox is recorded as the follow-up (§16).                                                                                                                                 |
| 3   | major    | The duplicate-extension rule blocked any save touching a server with a conflict already in the file.                                                                        | Accepted         | §8.2: only conflicts in the draft and not in the baseline block, keyed by `(extension, first, second)`. The `changed` closure is gone. New test for this case.                                                                                                                                                                                                                                                                                                                                   |
| 4   | major    | Availability and completion read the host (real cache, `PATH`, `JAVA_HOME`), so the tests depended on the machine. BDD scenario 1 asserted a status while css was selected. | Accepted         | D12, §9.5: `app.lsp_probe`, installed by bootstrap and injected by tests through `test_support`. The reviewer's parameter version cannot reach BDD, which opens the dialog by keys. Scenario 1 presses Down to fixture, and `System` is `"5000"`, not the default.                                                                                                                                                                                                                               |
| 5   | major    | Opening the dialog scanned `PATH` on the UI thread, and on Windows the scan offered DLLs and shims that LSP lookup rejects.                                                 | Accepted         | §7.3: a background `warm()` and a non-blocking `ready()`. §7.1: `lsp::launchable_name` keeps only `.exe` on Windows, so the hint and the Status row agree.                                                                                                                                                                                                                                                                                                                                       |
| 6   | minor    | The list went stale with AutoComplete off, auto-open took Enter, and one letter opened hundreds of `PATH` names.                                                            | Accepted         | Every edit refilters an open list. An auto-opened list is passive and leaves Enter to the form (D8, §5.4, §7.3). `PATH` names join from two characters (§7.2).                                                                                                                                                                                                                                                                                                                                   |
| 7   | minor    | With `preserve_order`, `Map::remove` is `swap_remove` and reorders keys, and `cargo tree` cannot show order reliance.                                                       | Accepted         | D6 and §9.2 use `shift_remove`. Order tests were added. The `cargo tree` check is replaced by an `rg` audit.                                                                                                                                                                                                                                                                                                                                                                                     |
| 8   | minor    | The mouse was not ignored: the wheel moves things, and the buttons looked clickable but did nothing.                                                                        | Accepted         | §5.6: a `dialog_button_key` arm for the form and the exit question, and the wheel documented. List-row and key-bar clicks are stated as unsupported (D11).                                                                                                                                                                                                                                                                                                                                       |
| 9   | minor    | The save notice described runtime behaviour that cannot happen while the dialog is open.                                                                                    | Accepted         | New notice (§10.3). The runtime fact is corrected in §2, and §14 and §16 are updated.                                                                                                                                                                                                                                                                                                                                                                                                            |
| 10  | minor    | The task lists named a missing `src/lsp.rs`, missed visibility work, and needed a `ui/dialog.rs` change for `↓`.                                                            | Accepted in part | Accepted: `mod install` is private, so §9.4 adds the `pub(crate) use` to `lsp.rs`, T5 lists that file and the second `inventory` re-export, and §13.4 notes that `inventory` is not in `UNEXERCISED`. Rejected: the spec never named `src/lsp.rs`. It names `lsp/install.rs`, which exists. Rejected: `Row::Field` has no `↓`. `render_row` draws `Row::Field` through `captioned_box`, which always pushes `DROP_MARKER` (`dialog.rs:534, 689`), so `ui/dialog.rs` needs no change (§4.2, T10). |
| 11  | minor    | Dead `ops::savesetup::run`, a per-field compare-and-set heavier than needed, and undo that nobody asked for.                                                                | Accepted         | T2 deletes the dead route instead of porting it. D4 checks the whole `lsp` subtree, which removes `Change.expected` and the conflict paths. Undo is dropped (D11, §12), following the PATH editor.                                                                                                                                                                                                                                                                                               |
| 12  | minor    | Strings stood in for enums and newtypes, and there were two AutoComplete settings.                                                                                          | Accepted         | `ReadOnly(ReadOnlyReason)`. The conflict path strings disappeared with finding 11 (`LspChanged` is a unit variant). `Change` is keyed by `LanguageId`, with `from_key` for loaded keys. `DocumentError` is `Clone` (`ErrorKind`, `JsonError`), so it can live in the overlay. D8 names `SettingsPage::AutoComplete` as the governing setting and updates its caption. The unread typed struct is out of scope (§16).                                                                             |
| 13  | minor    | On macOS, the OS can take both Ctrl+Space and Ctrl+Down.                                                                                                                    | Accepted         | Alt+Down is a third request key (D8, §5.3). §5.7 and the parity doc give the macOS defaults.                                                                                                                                                                                                                                                                                                                                                                                                     |
| 14  | minor    | Arguments accepted control characters, and mid-word quotes were undefined.                                                                                                  | Accepted         | D7 and §9.1 define quotes anywhere in an argument and reject control characters. The round-trip rows include `--x="a b"`. The form refuses to open on a loaded control character (§8.1).                                                                                                                                                                                                                                                                                                         |
| 15  | minor    | Edge cases in the write path: a dangling symlink, a `.bak` beside the canonical target that every writer shared, and a dropped hard-link rule.                              | Accepted         | D10 and §10.1 refuse a dangling link (the editor too, through the shared `real_target`). The `.bak` goes beside the link and is written only by dialog saves. Hard-linked, foreign-owned and unwritable files are refused, because the write must be atomic.                                                                                                                                                                                                                                     |
| 16  | minor    | `config::Settings::save` replaced page objects whole, dropping nested keys it does not know.                                                                                | Accepted         | §10.2: a recursive merge that sets only the keys the struct defines. A test covers `{"panel": {"future": 1}}`.                                                                                                                                                                                                                                                                                                                                                                                   |
| 17  | minor    | `DoesNotLoad` could not name the key, because it came from a `Value` without positions.                                                                                     | Accepted         | `loads` parses from bytes, so errors carry line and column, and the LSP deserializer adds `lsp.<language>: ` (§9.1, §10.1).                                                                                                                                                                                                                                                                                                                                                                      |
| 18  | minor    | The selection was an index, which could point at a different server after a save changed the table.                                                                         | Accepted         | `selected: Option<LanguageId>`, found again after a save or a Remove, and clamped when its language is gone (§9.3).                                                                                                                                                                                                                                                                                                                                                                              |

## 18. Implementation notes

Where the build departed from §15, and why.

| Task   | Planned                                                                                                         | Built                                                                                                                                     | Why                                                                                                                                                                                                                                                   |
| ------ | --------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| T5, T8 | T5 deletes `inventory`, and T8 replaces `ops/lsp_status.rs` and rewires the `lsp` command and the Commands row. | Both moved to T11, which switches the entry points and deletes the report and `inventory` together.                                       | Until T10 the overlay had no renderer. An entry point switched in T8 would have opened a blank dialog, and the old report needed `inventory` until then. T8–T10 carry expiring `expect(dead_code)` markers that fail the build once the code is used. |
| T5     | A `lsp_probe` field on `App`.                                                                                   | `app.host: Host { program_pause, lsp_probe }`.                                                                                            | Another field pushed `App::with_panels` over clippy's `too_many_lines`. Both fields describe the machine FFM runs on.                                                                                                                                 |
| T9     | A key map for the exit question.                                                                                | `popup/editor.rs` shares `ExitKey` / `exit_key` with the editor's own exit question.                                                      | One answer map for both Y/N/C questions (DRY).                                                                                                                                                                                                        |
| T10    | The read-only reason is on row 1, and the message row repeats it when a changing key is pressed.                | The message row also shows it from the moment the dialog opens.                                                                           | At 80 columns a long settings path cuts the reason off row 1, and then `line N column M` is not on screen.                                                                                                                                            |
| T12    | `capture_lsp_editor_review_screen` and the regenerated PNG.                                                     | The capture test is in the repo. The step that turns its buffer into `docs/images/lsp-inventory.png` (800×480) ran from a scratch script. | The repo has no buffer-to-PNG tool, and adding one for one screenshot is out of scope.                                                                                                                                                                |

## 19. Review of the build

An adversarial review of the built dialog (at `09f3b15`) raised 16 findings,
two of them twice. Each was reproduced against the code, with a regression
test that failed before its fix, before it was accepted or rejected.

| #    | Severity | Finding                                                                                                           | Verdict                         | What changed, or why not                                                                                                                                                                                                                                                                  |
| ---- | -------- | ----------------------------------------------------------------------------------------------------------------- | ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1    | blocker  | A no-break or ideographic space in Extensions sliced inside the character and panicked; the release build aborts. | Accepted                        | The word starts after the separator's last byte. Unit and key-level tests type U+00A0 and U+3000.                                                                                                                                                                                         |
| 2    | minor    | An added server removed and added again was saved as a field patch of the old entry.                              | Accepted                        | `patch::diff` writes it whole when it differs outside the owned fields, which only a remove and add can do.                                                                                                                                                                               |
| 3, 6 | major    | A notice never cleared, so `Saved to …` stood over new changes.                                                   | Accepted                        | A notice lasts one key (§4.1).                                                                                                                                                                                                                                                            |
| 4, 8 | major    | A stored `.zig` showed as `..zig` and OK refused it; a stored `a,b` was not reported.                             | Accepted                        | The line writes one dot; `Extension::parse` refuses a comma; the Problem row names what OK stores only when it would.                                                                                                                                                                     |
| 5    | minor    | Every save rewrote numbers through `f64`.                                                                         | Accepted, then superseded (§21) | serde_json `arbitrary_precision`; the suite passes with it, and no untagged or tagged type holds a float. An exponent is still written `1e+2`.                                                                                                                                            |
| 7    | major    | A click on the suggestion list pressed the hidden button beneath.                                                 | Accepted                        | `DialogMouse::covered` and `DialogTarget::Popup` (§5.6).                                                                                                                                                                                                                                  |
| 9    | minor    | The form and the exit question showed the list's key bar and legend.                                              | Accepted                        | Blank key bar; a legend per mode (§4.2).                                                                                                                                                                                                                                                  |
| 10   | minor    | A click on a form row did nothing, though the parity doc said it dragged.                                         | Accepted                        | Rows take clicks as in the Copy dialog (§5.6).                                                                                                                                                                                                                                            |
| 11   | minor    | The form and the exit question opened at the last drag offset.                                                    | Accepted                        | Opening either resets the dialog mouse.                                                                                                                                                                                                                                                   |
| 12   | minor    | Shift, Alt and Ctrl with a list or view key acted as the plain key.                                               | Accepted                        | A modified list or view key does nothing (§5).                                                                                                                                                                                                                                            |
| 13   | minor    | Strings, bools and positional counts where the rule asks for newtypes and enums.                                  | Accepted in part                | Accepted: a private `Accepted` built through the newtypes, `Drawn`, `RowFocus`, `DetailPane`, and `&LanguageId` throughout. Rejected: `Extension` in `Problem::DuplicateExtension`. The problem describes stored values, which may be invalid, and `Extension` can only hold a valid one. |
| 14   | minor    | The owned fields were copied in three places; `reset_or_remove_selected` named two jobs.                          | Accepted                        | `patch::with_owned_fields`, driven by `OwnedField`; `delete_selected`.                                                                                                                                                                                                                    |
| 15   | minor    | `writable()` read as a predicate but set a notice, and `prepare` dropped its result.                              | Accepted                        | `LspEditor::read_only_notice()` is pure and shared with the message row.                                                                                                                                                                                                                  |
| 16   | minor    | Test helpers were copied between modules.                                                                         | Accepted                        | `test_support::lsp_machine` holds `press`, `press_with`, `type_text` and `select`; the screen tests use `buffer_string`.                                                                                                                                                                  |

## 20. Mutation testing of the build

`cargo mutants --in-diff` over `git diff d78f47c HEAD`, the whole feature,
with `--jobs 6` and the repository's `.cargo/mutants.toml` (nextest,
`test-helpers` and `keepass`), run as the unprivileged `ffmguest` in a fresh
clone.

| Run   | Code at   | Mutants | Caught | Missed | Timeout | Unviable |
| ----- | --------- | ------- | ------ | ------ | ------- | -------- |
| First | `e10c3af` | 837     | 643    | 80     | 10      | 104      |
| Last  | `3abb726` | 839     | 721    | 8      | 6       | 104      |

The 80 misses were weak tests, not dead code. What pinned them:

- **Settings reader.** The capped read moved into `read_capped`, which takes
  any reader, so a test can hand it one byte more than the limit. A link loop
  is unreadable, not absent. As root, another user's file is refused with
  `NotOwned` and keeps its text and owner; `owned_by_us` is false for a
  root-owned path.
- **Completion.** An npm recipe's binary is offered as a command; the part of
  an extension after the caret is still offered; `ready()` answers with the
  scan `executables()` made. A `PATH` program now matches a known command by
  `program_file` alone: the plain comparison beside it was redundant on both
  systems.
- **Keys.** Del checks a reset server again; Up and Shift+Tab act in an active
  list; only an edit refreshes the suggestions; "No suggestions" answers a
  request and never typing; a click on a form row toggles only Enabled.
- **Start-up.** No test ran the program's own start-up, so the probe of the
  real `PATH` could vanish unseen. The pty suite opens the dialog in the real
  binary with a fixture server on `PATH`, and fails at once when `ffm` exits
  before its first frame.
- **Screen.** The tests looked for words, so the rules, the row heights, the
  message row, the detail pane's cut and the blue ground were unpinned. The
  layout tests compare every row of the documentation's synthetic dialog at
  80×24, 80×16, 80×15, 60×12, 3 to 8 rows and 4×3, and the F3 view at 80×24,
  7, 6 and 4 rows; `wrap`, extensions too long for their column, the focused
  control, a field's room and the dropdown's room below are tested directly.

One defect was found this way. Below seven rows the list screen drew rows over
each other: at 3 rows the settings and the message were written into the
bottom border, and at 4 they overwrote each other. A test that asks, at every
height from 3 to 24, that nothing reaches the bottom border or the key bar
failed, and `73a0769` fixed the layout (§4.1).

The six timeouts of the last run are tests with a 60-second deadline waiting
for a `PATH` scan that the mutant never starts. With `--timeout 150` all six,
and the two `bootstrap.rs` mutants, are caught.

Of the last run's eight misses, `fit`'s `>` → `==` is caught by the test of
extensions too long for their column, added after it. The other seven cannot
be caught on Linux:

| Mutant                                                               | Why it survives                                                                                                                                |
| -------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `file_write.rs`: `owned_by_us` → `false`; `has_other_names` → `true` | The `#[cfg(not(unix))]` bodies are not compiled on Linux. On Windows every settings save would be refused, which the update tests there catch. |
| `launch.rs`: `launchable_name` → `true` / `false`                    | `#[cfg(windows)]`; `only_a_launchable_program_is_offered_on_windows` catches both.                                                             |
| `popup/lsp_editor.rs`: `CONTROL \| ALT` → `CONTROL ^ ALT`            | Equivalent: the two flags share no bit.                                                                                                        |
| `ui/lsp_editor.rs` `split`: the key bar's `height: 1` deleted        | Equivalent: the bar is the last row, so a taller area reaches only rows below the screen, which are not drawn.                                 |
| `ui/lsp_editor/list.rs` `fit`: `>` → `>=`                            | Equivalent: a piece that fits exactly is kept whole by `head_cells` either way, and the next piece gets no room.                               |

## 21. Port onto settings.yaml

The dialog was built on a branch whose settings were `settings.json`. Main
meanwhile moved them to `settings.yaml` (one YAML document that the typed
settings and the Options pages share, three-way saves under
`settings.yaml.lock`), took the reliability lint set, and replaced
`ffm.sh` with `./ffm.py` and `mod.rs` files with `name.rs`. The port keeps
the dialog's behaviour, keys, layout and tests, and puts its save on main's
document instead of a second settings layer.

**One settings document, one writer path.** `config::document` gains `edit`,
which changes the file's own mapping under the same lock, and `update` and
`edit` share one private core, `rewrite`: parse the stored file, change a
copy, check that the copy still parses, and write it unless nothing changed.
F2 goes through `config::lsp::patch::Edit::save`: it writes only the owned
fields that changed (`enabled`, `command`, `args`, `extensions`) under
`lsp.<language>`, unsets a field that is back at the catalog's value, writes
an added server whole and drops a removed one, exactly as §10.3 did; the patch
now works on `serde_norway::Mapping`. After a save,
`config::Settings::adopt_saved_lsp` makes the saved servers both the session's
and its loaded snapshot, so the exit auto-save neither pins the catalog into
the file nor reads a later hand edit of `lsp` as a conflict. The session's
live input (`LiveInput::lsp_probe`) replaces the branch's `Host`; a replayed
recording gets an inert probe.

| Branch guarantee                                                                         | On main                                                                                                                                                                                                                                                                       |
| ---------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Symlink written through; dangling link, hard-linked file and another user's file refused | Kept, in `document::write`, so every writer (Save setup and the exit auto-save too) is held to them. The lock stays beside the link.                                                                                                                                          |
| Read-only file refused                                                                   | Main's `ensure_writable`, now run on the link's target.                                                                                                                                                                                                                       |
| Refused when `lsp` changed on disk while the dialog was open                             | Kept under the lock, comparing the typed `lsp` with the dialog's baseline; a respelling that loads as the same servers, and edits outside `lsp`, no longer refuse.                                                                                                            |
| The edited file must load                                                                | Kept for every writer: `rewrite` re-parses before writing.                                                                                                                                                                                                                    |
| A file that does not load is never rewritten; FIFO and 1 MiB guard                       | Main already had both.                                                                                                                                                                                                                                                        |
| Re-read before the rename (`ChangedWhileSaving`)                                         | Kept for every writer: `rewrite` reads the file again just before the write and refuses (`… changed while it was being saved; save again`) when it no longer holds what the save read. The lock keeps out only other FFMs; an editor, `sed -i` or a dotfiles sync takes none. |
| `settings.json.bak`                                                                      | Dropped: main keeps no backup, and a backup written by one writer alone is restored over the others' later saves. Discard and Del remain the dialog's recovery.                                                                                                               |
| Directory fsync after the rename                                                         | Dropped: settings and histories share one atomic write, which does not sync the directory.                                                                                                                                                                                    |
| Numbers keep their digits (`arbitrary_precision`)                                        | Dropped: YAML re-spells numbers anyway, the dialog writes none, and `arbitrary_precision` would corrupt a legacy JSON float on migration.                                                                                                                                     |
| JSON line/column errors                                                                  | YAML syntax errors carry line and column; wrong values carry their key path (`lsp: python: invalid type …`).                                                                                                                                                                  |
| One owner per top-level key                                                              | Replaced by main's `options:` key for Save setup and the typed pages for the exit save.                                                                                                                                                                                       |

Also removed with the port: the dead `ToolCommand::SaveSetup` route and
`ops::savesetup` (Shift+F9 is the menu's Save setup). The fish-stub ETXTBSY
flake that `3abb726` cured with a retry loop stays cured without a second
copy of it: main's `wait_until_executable` runs the stub once before the same
test uses it, and once one run starts no later one can hit the race. Main's
catalog gained `bat`, so the list starts there and the screen tests and the
TUI scenarios count one more row; the screenshot's fixture turns `bat` off, so
its first screen still shows a disabled, `missing` server and F3's first page
a configured executable. The TUI scenario's fake server is written under the
name lookup finds (`fixture-server.exe` on Windows).
