<!-- Purpose: Gives Claude Code repository-specific workflow and quality instructions. -->

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/claude-code) when working with code in this repository.

## Project Overview

FunnyFileManager (FFM) is a terminal-based dual-panel file manager inspired by Far Manager, written in Rust using the `ratatui` TUI library. It supports local filesystem navigation and AWS S3 integration.

## Build Commands

```bash
cargo build                    # Build the project
cargo run                      # Run the file manager
cargo test                     # Run tests
cargo test --features test-helpers  # Run tests with test helpers
cargo tarpaulin                # Run tests with coverage
```

## Project Structure

```
src/
├── main.rs                  # Entry point (thin wrapper calling lib::run())
├── lib.rs                   # Public API, event loop
├── bootstrap.rs             # Real-TTY entry (run/main_impl); coverage-excluded OS glue
├── action.rs                # Pure main-view key-to-Action mapping
├── command.rs               # Typed command parsing
├── models.rs                # Core data structures and domain enums
├── models/
│   ├── panel.rs             # Panel navigation and directory state
│   ├── editor.rs            # Built-in editor state
│   └── viewer.rs            # Built-in viewer state (file, encoding, offsets)
├── ui.rs                    # TUI layout, panels, status and command bars
├── ui/
│   ├── code.rs              # Highlighted, clipped text shared by editor and viewer
│   ├── overlays.rs          # Modal overlay rendering
│   └── viewer.rs            # Far-style full-screen viewer with its status line
├── popup.rs                 # Strictly modal overlay input
├── ops/                     # Canonical command, file, shell, viewer/MCP operations
├── handlers/
│   └── mod.rs               # Backward-compatible re-export facade
├── terminal.rs              # RAII terminal session and editor suspension
├── utils.rs                 # Utility functions (format_size, copy_dir_all, etc.)
├── platform.rs              # Safe wrappers for OS calls std lacks (uids, access, drives)
├── editor.rs                # Built-in editor opening (F4) + external editor (Alt+F4)
├── s3.rs                    # AWS S3 integration (browse/upload/download)
└── windows.rs               # Windows-specific diagnostics and compatibility

tests/
├── integration_test.rs          # Core integration tests
├── handlers_test.rs             # Handler function tests
├── handlers_mutation_tests.rs   # Mutation-killing handler tests
├── models_mutation_tests.rs     # Mutation-killing model tests
├── mutation_tests.rs            # General mutation tests
├── editor_test.rs               # Editor integration tests
├── far_manager_hotkeys_test.rs  # Far Manager hotkey compatibility tests
├── ui_tests.rs                  # UI rendering tests
├── navigation_features_test.rs  # Navigation feature tests
├── matching_test.rs             # Quick search / matching tests
├── virtual_path_matrix_test.rs  # VirtualPath (Local/S3) parsing tests
├── windows_behavior_test.rs     # Windows behavior tests
├── comprehensive_test.rs        # Comprehensive scenario tests
├── coverage_100_test.rs         # Tests targeting 100% coverage
├── coverage_gaps_test.rs        # Tests for coverage gaps
├── coverage_smoke_test.rs       # Coverage smoke tests
├── extra_coverage_test.rs       # Extra coverage tests
├── increased_coverage_test.rs   # Incremental coverage tests
├── targeted_coverage_tests.rs   # Targeted coverage tests
├── additional_tests.rs          # Additional edge case tests
├── nav_debug_test.rs            # Navigation debugging tests
├── generated_format_size_1000_test.rs  # Generated format_size tests
└── generated_utils_variety_test.rs     # Generated utils tests

benches/
└── utils_bench.rs  # Benchmarks for utility functions

mcp/
├── Cargo.toml      # MCP server crate manifest
├── Cargo.lock      # MCP server dependency lock
├── bench_mcp.rs    # MCP server benchmark script
└── MCP.md          # MCP server documentation

docs/
└── best-practices.md  # Testing and development best practices

scripts/
├── fetch_far_manager_docs.ps1  # Download Far Manager reference documentation
└── k9s-eks.sh                  # k9s + Amazon EKS helpers
```

## Repository Root Files

| File | Description |
|------|-------------|
| `Cargo.toml` | Rust crate manifest (dependencies, features, metadata) |
| `Cargo.lock` | Pinned dependency versions |
| `CLAUDE.md` | Instructions for Claude Code AI assistant |
| `AGENTS.md` | Instructions for OpenAI Codex agent |
| `GEMINI.md` | Instructions for Google Gemini agent |
| `README.md` | Project README |
| `PRESENTATION.md` | Marp slide deck for project presentation |
| `.gitignore` | Git ignore rules |
| `.mcp.json` | MCP server configuration for AI IDEs |
| `rustfmt.toml` | Rust formatter settings |
| `tarpaulin.toml` | cargo-tarpaulin coverage configuration |
| `ffm.sh` | Build/run/test script for Unix |
| `ffm.cmd` | Build/run/test script for Windows |
| `commit.sh` | Auto-commit helper script |
| `cron-run.sh` | Cron job: Claude Code review → build → test → push |
| `install-mcp.sh` | Install FFM MCP server for all AI IDEs |
| `create-desktop-shortcut.sh` | Create desktop shortcut (Linux) |
| `create-desktop-shortcut.ps1` | Create desktop shortcut (Windows/PowerShell) |
| `create_icon.cmd` | Create desktop icon (Windows/CMD) |

### Hidden Config Directories

| Directory | Description |
|-----------|-------------|
| `.cargo/config.toml` | Cargo build settings: shared `target/`, mold linker on Linux, LLD on Windows, no compiler wrapper (sccache is opt-in via `RUSTC_WRAPPER`); parallelism is Cargo's default |
| `.cargo/mutants.toml` | cargo-mutants configuration: exclusions, features, nextest test runner (read automatically) |
| `.claude/` | Claude Code slash commands and local settings |
| `.gemini/` | Gemini IDE settings |
| `.junie/` | JetBrains Junie MCP config |
| `.kiro/` | AWS Kiro MCP config |
| `.github/workflows/tests.yml` | GitHub Actions CI pipeline |

---

## Module Reference

### main.rs
Entry point that calls `funny_file_manager::run()`.

### lib.rs
Main application coordination and event loop. An active overlay receives every
key before the main-view action resolver. Terminal setup and restoration are
owned by RAII guards in `terminal.rs`.

| Function | Description |
|----------|-------------|
| `run()` | Public entry point, wraps `main_impl()` with logging |
| `run_with(main_runner)` | Wrapper that logs start/end and errors |
| `main_impl()` | Creates App and terminal, enters `TerminalSession`, runs the event loop, then finishes cleanup |
| `run_app(terminal, app)` | Main event loop wrapper |
| `run_app_with_event_reader(terminal, app, read_event)` | Event loop with custom event reader |
| `log_action(action)` | Append to the opt-in action log |
| `log_error(error)` | Append to the opt-in error log |

Runtime logging is disabled unless `FFM_LOG=1`. Enabled logs are written under
the per-user temporary directory, never the launch directory; character key
payloads are redacted. Logging tests use explicit temporary paths.

**Main-view actions (resolved in `action.rs`):**
- `Q` / `F10` / `Ctrl+C` - Quit
- `Up` / `Down` - Navigate entries
- `Enter` - Open directory/file
- `Tab` - Switch panels
- `Left` / `Right` - Select panel (Brief/Medium: move the cursor one column)
- `Ctrl+1` … `Ctrl+4` - Brief / Medium / Full / Wide view (`Ctrl+5` … `Ctrl+0`: the detail views)
- `Home` / `End` - Jump to first/last
- `PgUp` / `PgDn` - Move by one panel page
- `F1` - Toggle help
- `F2` - Open the user command menu
- `F3` - View file
- `F4` - Edit file (built-in editor)
- `Alt+F4` - Edit file in external editor
- `Shift+F4` - Create new file
- `F5` - Copy
- `F6` - Move
- `F7` - Mkdir
- `F8` - Delete (with confirmation)
- `F9` - Open the menu bar
- `F11` - Show MCP/plugin status
- `F12` - Screens list (panels / console; `Ctrl+O` keeps the direct toggle)
- `Alt+F1` / `Alt+F2` - Drive selection
- `Ctrl+O` - Toggle console mode
- `Ctrl+U` - Swap panels
- `Ctrl+R` - Refresh panels
- `Ctrl+\` - Go to root
- `:` - Open command line
- `Insert` / `Space` - Toggle mark
- `Alt+letter` - Quick search

**Structs:**
- `WritableTestBackend` - Test backend implementing `Write` + `Backend` traits

---

### models.rs
Core data structures for the application.

| Struct/Enum | Fields | Description |
|-------------|--------|-------------|
| `App` | `left_panel`, `right_panel`, `active_panel`, `overlay`, `bookmarks`, `should_quit`, `status_message`, `quick_search` | Main application state |
| `Panel` | `path`, `entries`, `state`, `marked`, `sort_mode` | File panel state |
| `DirEntry` | `name`, `kind`, `size`, `modified`, `executable`, `links`, `owner`, `allocated`, `created`, `accessed`, `attributes`, `description` | Single directory entry (`models/entry.rs`) |
| `Viewer` | `title`, `lines`, `scroll`, `col`, `page`, `file` | Read-only file viewer state; `file` is `Some(ViewedFile { path, size, encoding })` for a real file, `None` for a report |
| `Overlay` | `None`, `Help`, `Viewer`, `Editor`, `DriveSelection`, `NewFileInput`, `CommandLine`, `DeleteConfirm`, `QuitConfirm`, `NamePrompt`, `MaskPrompt`, `CopyPrompt`, `Information`, `Menu`, `Wsl`, and one per F9 menu dialog: `WipeConfirm`, `ArchivePrompt`, `ExtractPrompt`, `ArchiveCommands`, `Attributes`, `ApplyCommand`, `Describe`, `FindFile`, `CommandHistory`, `FindFolder`, `ViewHistory`, `FolderHistory`, `UserMenu`, `Associations`, `FolderShortcuts`, `Filter`, `Screens`, `Tasks`, `Hotplug`, `SortModes`, `Settings` | Mutually exclusive modal state |
| `SortMode` | `Name`, `Extension`, `Size`, `ModificationTime`, `Unsorted`, `CreationTime`, `AccessTime`, `Descriptions`, `Owner` | Sorting mode enum; header letters `n e s m u c a d o` |
| `PanelSide` | `Left`, `Right` | Panel identifier |
| `DriveEntry` | `Local(PathBuf)`, `S3Profile(String)` | Drive type enum |

**App Methods:**
| Method | Description |
|--------|-------------|
| `new()` | Create new App with both panels at current directory |
| `get_active_panel()` | Get mutable reference to active panel |
| `get_inactive_panel()` | Get mutable reference to inactive panel |
| `switch_panel()` | Toggle between left and right panels |
| `swap_panels()` | Swap contents of both panels (Ctrl+U) |
| `get_drives()` | Get available drives (Windows: A-Z, Unix: /) |
| `get_all_drives()` | Get local drives + S3 profiles |

**Panel Methods:**
| Method | Description |
|--------|-------------|
| `new(path)` | Create panel at given path |
| `load_entries()` | Reload directory contents |
| `next()` / `previous()` | Navigate selection with wraparound |
| `enter()` | Enter selected directory |
| `get_selected_entry()` | Get currently selected entry |
| `selected_path()` | Get full path of selection |
| `toggle_mark_selected()` | Mark/unmark selected file |
| `clear_marks()` | Clear all marks |
| `find_first_matching(prefix)` | Quick search - find first match |
| `find_next_matching(prefix)` | Quick search - find next match |
| `ensure_selection()` | Ensure valid selection after reload |
| `cycle_sort_mode()` | Switch between sort modes |
| `action_entry_names()` | Get names for file operations |

---

### ui.rs
TUI rendering functions using ratatui.

| Function | Description |
|----------|-------------|
| `ui(frame, app)` | Main render function - draws entire UI |
| `render_panel(frame, panel, area, is_active)` | Render a single file panel |
| `render_header(frame, area)` | Render header bar |
| `render_status(frame, app, area)` | Render status bar with file info |
| `render_command_bar(frame, area)` | Render F-key command bar |
| `render_overlays(frame, app)` | Render all overlay windows |
| `centered_rect(percent_x, percent_y, rect)` | Calculate centered rectangle |

**Color Scheme:**
- `FAR_BLUE`: `RGB(0, 0, 128)` - Background
- Active border: Cyan + Bold
- Inactive border: Blue
- Directories: White + Bold
- Files: Cyan
- Selection: Black on Cyan

**Layout:**
```
+------------------------------------------+
| Header (1 line)                          |
+-------------------+----------------------+
| Left Panel        | Right Panel          |
| (50%)             | (50%)                |
+-------------------+----------------------+
| Status Bar (2 lines)                     |
+------------------------------------------+
| Command Bar (2 lines)                    |
+------------------------------------------+
```

**Overlays (mutually exclusive):**
- Help overlay (F1)
- Viewer overlay (F3) - full screen like Far's: a teal status line on row 0
  (full path, `t`, encoding, size, `Col N`, percent, clock), the
  syntax-highlighted file below it
- Editor overlay (F4) - built-in text editor
- Drive selection (Alt+F1/F2)
- Delete confirmation (F8)
- New file input (Shift+F4)
- Command line (`:`)
- Console mode (Ctrl+O) - hides panels
- F9 menu dialogs, one `Overlay` variant each (`src/models/overlay.rs`):
  Wipe (Alt+Del), Add to archive / Extract / Archive commands
  (Shift+F1/F2/F3), File attributes (Ctrl+A), Apply command (Ctrl+G),
  Describe files (Ctrl+Z), Find file (Alt+F7), History (Alt+F8), Find folder
  (Alt+F10), File view history (Alt+F11), Folders history (Alt+F12), User
  menu (F2), File associations, Folder shortcuts, File panel filter (Ctrl+I),
  Screens (F12), Task list (Ctrl+W), Hotplug devices, Sort modes (Ctrl+F12),
  and one Settings dialog per Options page. Each is wired as
  `ops::<feature>::open` → `popup::<feature>::process` →
  `ui::<feature>::render`; the placeholders say "Coming in this release".

---

### action.rs

Pure main-view input DSL. `resolve(key)` maps `KeyEvent` to a typed `Action`;
it is not called while an overlay is active.

### popup.rs

`process_popup_input(app, key)` handles the active `Overlay`. Every key is
consumed while an overlay is active, including unsupported command-line keys.
It calls canonical operations through `crate::ops`, never through `handlers`.

### ops/

Canonical implementation owner for command, file, shell, viewer, and
MCP-status operations.

- `ops/dispatch.rs` — typed command execution
- `ops/file.rs` — create, copy, move, mkdir, and delete
- `ops/shell.rs` — platform shell execution
- `ops/viewer.rs` — file viewer and MCP status

### handlers/mod.rs

Backward-compatible re-exports from `ops` and `popup`. It contains no
production operation implementation.

**Command-line Commands:**
| Command | Arguments | Description |
|---------|-----------|-------------|
| `cd` | `<path>` | Change directory |
| `pwd` | - | Show current path |
| `sort` | `name\|ext\|size\|mtime\|unsorted\|ctime\|atime\|desc\|owner` | Change sort mode |
| `bookmark` | `set\|go <0-9>` | Manage bookmarks |
| `mark` | `all` | Mark all files |
| `unmark` | - | Clear all marks |
| `help` | - | Show help overlay |
| `mcp` | - | Show MCP status |
| `winmsg` / `winperf` | - | Show Windows diagnostics |
| `quit` | - | Exit application |

Unknown verbs fall back to the platform shell. Modal editing and submission are
owned by `popup`, parsing by `command`, and execution by `ops/dispatch`.

There is no search or bookmark overlay: bookmarks are reached only through
`:bookmark set|go <0-9>`, and incremental search only through `Alt+letter`.

---

### utils.rs
Utility functions for file operations.

| Function | Description |
|----------|-------------|
| `format_size(bytes)` | Convert bytes to human-readable (B/KB/MB/GB) |
| `unique_destination(base, filename)` | Generate unique filename with `_N` suffix |
| `copy_dir_all(src, dst)` | Recursive directory copy |
| `refresh_panels(app)` | Reload both panels |

**Size Thresholds:**
- KB: 1,024 bytes
- MB: 1,048,576 bytes
- GB: 1,073,741,824 bytes

---

### editor.rs
Built-in editor opening (F4) and external editor integration (Alt+F4).

| Function | Description |
|----------|-------------|
| `handle_edit_builtin(app)` | F4 - Open file in built-in editor overlay |
| `handle_edit(terminal, app)` | Alt+F4 - Open file in external editor |
| `handle_edit_impl(terminal, app, run_editor)` | Unit/coverage seam with an injected editor runner |
| `handle_edit_test_mode(app, behavior)` | Explicit integration-test helper |

**External Editor Selection Priority (Alt+F4):**
1. VS Code (`code -w -n`) if available
2. `$EDITOR` environment variable
3. `nano` as fallback

**Test Helpers:**
- `TestEditorBehavior` enum: `Success`, `FailLaunch`, `ExitError`

Enabling `test-helpers` exposes explicit helpers; it does not replace
`handle_edit`, `run`, or `main_impl`.

---

### s3.rs
AWS S3 integration for cloud storage browsing.

| Struct | Description |
|--------|-------------|
| `AwsProfile` | AWS profile from `~/.aws/credentials` |
| `S3Bucket` | S3 bucket info |
| `S3Object` | S3 object (file or folder) |
| `S3Client` | S3 client wrapper with blocking operations |
| `VirtualPath` | Union type for Local or S3 paths |

**S3Client Methods:**
| Method | Description |
|--------|-------------|
| `new(profile_name)` | Create client for AWS profile |
| `list_buckets()` | List all S3 buckets |
| `list_objects(bucket, prefix)` | List objects in bucket |
| `download_object(bucket, key, local_path)` | Download file from S3 |
| `upload_object(bucket, key, local_path)` | Upload file to S3 |
| `delete_object(bucket, key)` | Delete S3 object |
| `copy_object(src_bucket, src_key, dst_bucket, dst_key)` | Copy within S3 |

**VirtualPath:**
| Variant | Description |
|---------|-------------|
| `Local(PathBuf)` | Local filesystem path |
| `S3 { profile, bucket, prefix }` | S3 location |

**Path Parsing:**
- `s3://bucket/prefix` - Default profile
- `s3://profile@bucket/prefix` - Named profile

**Utility Functions:**
| Function | Description |
|----------|-------------|
| `list_aws_profiles()` | Get profiles from `~/.aws/credentials` |
| `dirs_next_candidate()` | Get home directory path |

---

## Key Architecture Decisions

- **File size limit**: no file in `src/` may exceed 1000 lines (enforced by `tests/file_line_limit_test.rs`); split files over ~500 lines into modules. Unit tests live in `<module>/tests*.rs` child modules.
- **No private data**: `tests/private_data_lint_test.rs` scans every tracked file for personal e-mails, machine-specific absolute paths, credential-shaped strings and public IPs. Fixture paths use `/home/bob`, `/srv/x`, `example.com`. Site-specific words live outside the repo in `~/.config/ffm/private-words.txt`, never in the test.

- **Unsafe boundary**: `unsafe_code` is denied; items opt in with `#[allow(unsafe_code)]` only in files `tests/unsafe_boundary_test.rs` allowlists. Shared OS calls go through `src/platform`.
- **Canonical operations**: `src/ops/` owns command, file, shell, viewer, and MCP-status implementations; `handlers` is a compatibility facade
- **Modal-first input**: `popup::process_popup_input` consumes active-overlay keys before `action::resolve`
- **Terminal RAII**: `TerminalSession` and `TerminalSuspension` own best-effort restoration, including partial transitions
- **Feature-invariant runtime**: `test-helpers` exposes seams without selecting alternate runtime implementations
- **Far Manager UX**: Dark blue background (RGB 0,0,128), cyan text, F-key commands
- **Dual-panel layout**: Left/right panels with Tab switching
- **Overlay system**: Mutually exclusive modal overlays for dialogs
- **Selection preservation**: Maintains selection after panel reload
- **Rename-with-fallback**: Tries atomic rename, falls back to copy+delete
- **Unique destinations**: Never overwrites, adds `_N` suffix
- **Testable design**: Injection points for mutation testing

## Testing

Tests use `tempfile::TempDir` for filesystem isolation. Test modes:
- `#[cfg(test)]` - Standard unit tests
- `#[cfg(tarpaulin)]` - Coverage testing
- `#[cfg(feature = "test-helpers")]` - Additional integration tests and explicit helpers

**Injection Points:**
- `FORCE_RENAME_FAILURE` - In-memory move-fallback control
- `FORCE_COPY_DIR_ALL_STEP` - In-memory recursive-copy failure control
- `FORCE_REFRESH_ERROR` - In-memory panel-refresh failure control

These controls are cfg-gated test state, not environment variables.

## Dependencies

| Crate | Purpose |
|-------|---------|
| `ratatui` | TUI framework |
| `crossterm` | Terminal handling |
| `aws-sdk-s3` | AWS S3 client |
| `aws-config` | AWS configuration |
| `tokio` | Async runtime for S3 |
| `tempfile` | Test temporary directories |

## Documentation

- `docs/knowledge-base/02-architecture/module-structure.md` - Current module map
- `docs/specifications/command-popup-and-operations-modules.md` - Operation and modal boundaries
- `docs/kdbx-usage.md` - How to use the TUI vault browser and the MCP `kbdx` tool

---

## Far Manager Hotkey Reference

Canonical Far Manager hotkeys with FFM implementation status.
Legend: ✅ implemented · ❌ not implemented · ⚠️ partial/different

### Quit
| Key | Far Manager Action | FFM |
|-----|--------------------|-----|
| `F10` | Quit | ✅ |
| `Ctrl+C` | Quit | ✅ |
| `Q` | (FFM-specific quit) | ✅ |

### Navigation
| Key | Far Manager Action | FFM |
|-----|--------------------|-----|
| `Up` / `Down` | Move cursor | ✅ |
| `PgUp` / `PgDn` | Page up / Page down in panel | ✅ |
| `Home` / `End` | First / Last entry | ✅ |
| `Backspace` | Go to parent directory | ❌ |
| `Ctrl+PgUp` | Go to parent directory | ❌ |
| `Ctrl+PgDn` | Enter directory / archive | ❌ (Enter works) |
| `Enter` | Execute file / Enter directory | ✅ |
| `Tab` | Switch active panel | ✅ |
| `Left` / `Right` | Move one column in multi-column views; one page otherwise | ⚠️ (Brief/Medium: move a column; other views: switch panel) |
| `Alt+letter` | Quick search (jump to name) | ✅ |
| `Ctrl+\` | Go to root directory | ✅ |

### F-Key File Commands
| Key | Far Manager Action | FFM |
|-----|--------------------|-----|
| `F1` | Help | ✅ |
| `F2` | User menu | ⚠️ (opens the available Commands menu) |
| `F3` | View file (internal viewer) | ✅ |
| `Shift+F3` | View file (external viewer) | ❌ |
| `F4` | Edit file (built-in editor) | ✅ |
| `Shift+F4` | Create new file | ✅ |
| `F5` | Copy to other panel | ✅ |
| `Shift+F5` | Copy with rename | ❌ |
| `F6` | Move / Rename to other panel | ✅ |
| `Shift+F6` | Rename in place | ❌ |
| `F7` | Create directory (mkdir) | ✅ |
| `Alt+F7` | Find file | ⚠️ (dialog opens; search arrives with its feature) |
| `F8` | Delete (with confirmation) | ✅ |
| `Shift+F8` | Wipe file (overwrite + delete) | ❌ |
| `F9` | Activate menu bar | ✅ |
| `F10` | Quit | ✅ |
| `F11` | Plugin commands | ✅ (MCP/plugin status) |
| `F12` / `Ctrl+Tab` | Switch screens | ⚠️ (F12: Screens list; Ctrl+Tab not bound) |

### Alt+F-Key Commands
| Key | Far Manager Action | FFM |
|-----|--------------------|-----|
| `Alt+F1` | Drive menu for left panel | ✅ |
| `Alt+F2` | Drive menu for right panel | ✅ |
| `Alt+F4` | Edit file in external editor | ✅ |
| `Alt+F7` | Find file | ⚠️ (dialog opens; search arrives with its feature) |
| `Alt+F8` | Command history | ⚠️ (dialog opens; list arrives with its feature) |
| `Alt+F9` | Maximize / restore window | ⚠️ (Video mode: reports in the status bar) |
| `Alt+F10` | Folder tree / find folder | ⚠️ (dialog opens; tree arrives with its feature) |
| `Alt+F11` | View and edit history | ⚠️ (dialog opens; list arrives with its feature) |
| `Alt+F12` | Folder history | ⚠️ (dialog opens; list arrives with its feature) |

### Ctrl+F-Key Sort Commands
| Key | Far Manager Action | FFM |
|-----|--------------------|-----|
| `Ctrl+F1` | Hide / show left panel | ✅ |
| `Ctrl+F2` | Hide / show right panel | ✅ |
| `Ctrl+F3` | Sort by name | ✅ |
| `Ctrl+F4` | Sort by extension | ✅ |
| `Ctrl+F5` | Sort by modification time | ✅ |
| `Ctrl+F6` | Sort by size | ✅ |
| `Ctrl+F8` | Sort by description | ✅ |
| `Ctrl+F9` | Sort by creation time | ✅ |
| `Ctrl+F10` | Sort by access time | ✅ |
| `Ctrl+F11` | Unsorted (original order) | ✅ |
| `Ctrl+F12` | Sort modes menu | ✅ (modes, reverse order, `+` `-` `*`) |

### Marking / Selection
| Key | Far Manager Action | FFM |
|-----|--------------------|-----|
| `Insert` | Toggle mark, move to next | ✅ |
| `Space` | Toggle mark, move to next | ✅ |
| `Numpad+` (`Gray+`) | Select files by mask | ❌ |
| `Numpad-` (`Gray-`) | Deselect files by mask | ❌ |
| `Numpad*` (`Gray*`) | Invert selection | ❌ |
| `Ctrl+A` | Select all / change attributes | ✅ (Far's attributes dialog: permissions, times, subfolders) |
| `Ctrl+M` | Restore previous selection | ❌ |

### Panel Operations
| Key | Far Manager Action | FFM |
|-----|--------------------|-----|
| `Ctrl+U` | Swap panel contents | ✅ |
| `Ctrl+O` | Toggle console (hide panels) | ✅ |
| `Ctrl+R` | Refresh / re-read panel | ✅ |
| `Ctrl+1` | Brief view: three name columns | ✅ |
| `Ctrl+2` | Medium view: two name columns | ✅ |
| `Ctrl+3` | Full view: name, size, date, time | ✅ |
| `Ctrl+4` | Wide view: name, size | ✅ |
| `Ctrl+L` | Toggle info panel | ❌ |

### Clipboard & Command Line
| Key | Far Manager Action | FFM |
|-----|--------------------|-----|
| `Ctrl+Enter` | Insert filename into command line | ❌ |
| `Ctrl+F` | Insert full path into command line | ❌ |
| `Ctrl+Ins` | Copy selected filenames to clipboard | ❌ |
| `Ctrl+Shift+Ins` | Copy filenames to clipboard (always) | ❌ |
| `Ctrl+Alt+Ins` | Copy UNC names to clipboard | ❌ |
| `Alt+Shift+Ins` | Copy full paths to clipboard | ❌ |
| `Ctrl+End` | Command history completion | ❌ |

### Miscellaneous
| Key | Far Manager Action | FFM |
|-----|--------------------|-----|
| `Alt+Del` | Wipe file (secure overwrite) | ✅ (zero-fill, truncate, rename, remove; directories recurse) |
| `Ctrl+G` | Run command on selected files | ✅ (Far's `!` symbols; one run per marked entry, echoed to the Ctrl+O console) |
| `Ctrl+W` | Task list | ⚠️ (dialog opens; the list arrives with its feature) |
| `Ctrl+Z` | Describe file | ⚠️ (prompt opens; writing arrives with its feature) |
| `Shift+F1` | Create archive | ❌ |
| `Shift+F2` | Extract archive | ❌ |
| `Shift+F10` | Show last menu command | ❌ |

### Overlay / Dialog Keys (FFM-specific)
| Key | Context | Action |
|-----|---------|--------|
| `Esc` | Any overlay | Close / cancel |
| `Y` / `Enter` | Delete confirm | Confirm delete |
| `N` | Delete confirm | Cancel delete |
| `Up` / `Down` | Viewer | Scroll one line (the last line stops on the bottom row) |
| `PgUp` / `PgDn` | Viewer | Scroll one screen |
| `Home` / `End` | Viewer | First / last screen |
| `Left` / `Right` | Viewer | Pan one column (Left stops at column 0) |
| `Ctrl+Left` / `Ctrl+Right` | Viewer | Pan twenty columns (Far's step) |
| `Esc` / `Enter` / `F3` / `F10` | Viewer | Close |
| `:bookmark go <0-9>` | Command line | Jump to bookmark slot |
| `:bookmark set <0-9>` | Command line | Set bookmark slot |
| `Up` / `Down` / `Enter` | Drive selector | Navigate / select drive |

---

## How to Work With This Project (Lightweight RAG)

This project uses a shared knowledge base located at:

**`docs/knowledge-base/`**

**Recommended workflow:**
1. At the start of a session, read `docs/knowledge-base/00-index.md`.
2. When you need information on a topic, read the specific module file(s) from the knowledge base.
3. Use your file reading capabilities to pull in only relevant content.

High-priority modules:
- `03-rust-best-practices/type-system.md` (core philosophy — enums and newtypes)
- `02-architecture/module-structure.md`
- `05-testing/testing-strategy.md`

See `AGENTS.md` for the general instructions that apply to all LLMs regarding the knowledge base.

## Slash Commands (Skills)

| Command | What it does |
|---------|-------------|
| `/build` | Build project, fix warnings |
| `/check` | Full quality gate: fmt → clippy → build → test |
| `/clippy` | Run clippy and auto-fix all warnings |
| `/commit` | Stage, commit, rebase, push |
| `/deps` | Audit and update dependencies |
| `/doc` | Build docs and run doc tests |
| `/fix <error>` | Find and fix a specific bug |
| `/fmt` | Format all code with cargo fmt |
| `/github` | Check CI pipeline and fix failures |
| `/mutate` | Run mutation testing suite |
| `/profile` | Find performance bottlenecks |
| `/refactor <file>` | Refactor code to Rust best practices |
| `/release` | Build release binary |
| `/review` | Review recent changes for quality issues |
| `/scaffold <feature>` | Scaffold a new feature |
| `/test` | Run tests and improve coverage to 100% |
