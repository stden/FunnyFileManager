<!-- Purpose: Maps each major module to its responsibility in the codebase. -->

# Module Structure

FunnyFileManager uses a thin binary, a library-owned event loop, and focused
state, UI, input, operation, and lifecycle modules.

## Entry and Controller

### Binary (`src/main.rs`)

- Thin entry point that calls `funny_file_manager::run()`.

### Core Library (`src/lib.rs`)

- Constructs `App` and the Ratatui terminal.
- Runs `run_app_with_event_reader`.
- Routes every key to an active overlay before main-view action resolution.
- Applies typed `Action` values and coordinates UI rendering.
- Delegates application terminal setup and cleanup to `src/terminal.rs`.

The `test-helpers` feature exposes explicit utilities; it does not select a
different `run`, `main_impl`, event reader, or editor implementation.

### Terminal Lifecycle (`src/terminal.rs`)

- `TerminalOps` abstracts individual terminal transitions.
- `CrosstermOps` performs production Crossterm operations.
- `TerminalSession` owns raw mode, alternate screen, mouse capture, and
  cursor restoration for the application.
- `TerminalSuspension` temporarily restores the host terminal while an
  external editor runs, then re-enters the TUI.
- Partial transitions are conservatively rolled back; cleanup continues after
  a failed step, and application-session cursor restoration runs last.

## State and Presentation

### Data Layer (`src/models.rs`, `src/models/`)

- `App`, `DirEntry`, `Viewer`, and domain enums such as `SortMode`,
  `PanelSide`, `DriveEntry`, `Overlay`, and `BookmarkSlot`
- `src/models/panel.rs` — directory listing, navigation, selection, and marking
- `src/models/editor.rs` — built-in editor state and editing operations

### Presentation (`src/ui.rs`, `src/ui/`)

- Ratatui layout and rendering
- Panels, status bar, command bar, and modal overlays

## Input and Operations

### Main-View Actions (`src/action.rs`)

- Pure `KeyEvent` to `Action` mapping for the non-modal main view
- No terminal or filesystem I/O

### Command Representation (`src/command.rs`)

- `Command` enum for the `:` command language
- `Command::parse` as the syntax and validation authority
- Shared command metadata and path normalization support completion and dispatch
  without duplicating command syntax.

### Popup Input (`src/popup.rs`)

- `process_popup_input(app, key)` handles every `Overlay` variant, including the
  built-in editor and command-line completion menu.
- Any active overlay consumes every key; unsupported command-line keys leave
  the buffer unchanged.
- Calls canonical operation functions through `crate::ops`, never through the
  compatibility facade.
- Still owns some synchronous modal I/O; pure effect extraction is not part of
  the current foundation.

### Canonical Operations (`src/ops/`)

- `src/ops/mod.rs` — operation facade
- `src/ops/dispatch.rs` — typed command execution
- `src/ops/file.rs` — create, copy, move, mkdir, and delete
- `src/ops/shell.rs` — bounded, cancellable shell process execution
- `src/ops/viewer.rs` — file viewing and MCP status viewing

This directory is the sole implementation owner for command, file, shell,
viewer, and MCP-status operations.

### Compatibility Facade (`src/handlers/mod.rs`)

- Re-exports stable operation paths from `ops`.
- Re-exports `process_popup_input` from `popup`.
- Keeps hidden compatibility submodules and historical tests.
- Contains no production operation implementation.

See
`docs/specifications/command-popup-and-operations-modules.md`
for the detailed boundary contract.

Dependency direction:

```text
lib ───────────────> ops
 │                    ▲
 └─> popup ───────────┘

handlers facade ──> ops
                 └─> popup
```

## Supporting Modules

- `src/editor.rs` — built-in F4 opening and external Alt+F4 process integration
- `src/utils.rs` — size formatting, unique destinations, recursive copy, and
  panel refresh
- `src/complete.rs` — fuzzy command/path candidates for the command-line
  dropdown
- `src/status.rs` — typed status messages
- `src/s3.rs` — optional AWS S3 support
- `src/keepass.rs` — optional KeePass support
- `src/windows.rs` — Windows-specific behavior and diagnostics

## Testing Support

- `src/test_backend.rs` — `WritableTestBackend`, exposed under test-related cfg
- `src/test_support.rs` — explicit test utilities
- `run_app_with_event_reader` — injected event source for event-loop tests
- `handle_edit_test_mode` — explicit external-editor result seam
- `tests/` — integration, behavior, architecture, and mutation tests

File-operation fault injection uses cfg-gated in-memory controls in
`src/ops/file.rs` and `src/utils.rs`; it is not configured through environment
variables. Runtime logging is opt-in through `FFM_LOG=1`, writes under the
temporary directory, and redacts character key payloads. Logging tests pass
explicit temporary paths instead of changing the process-wide current
directory.

## Key Design Decisions

- `main.rs` remains wiring only.
- A single `Overlay` enum makes simultaneous modal states unrepresentable.
- Modal input never falls through to `Action::resolve`.
- `ops` has one-way dependencies and one canonical implementation per
  operation.
- RAII guards own terminal transitions.
- `test-helpers` changes API visibility and test availability, not runtime
  semantics.

## When Adding Code

- Put new command, file, shell, viewer, or MCP-status behavior in the
  corresponding `src/ops/` child module.
- Keep `handlers` as a compatibility facade.
- Put modal key behavior in `popup`; put non-modal key mapping in `action`.
- Prefer focused modules over growing `lib.rs`.

## File Size Rule

- Hard limit: 1000 lines per file under `src/`, enforced by
  `tests/file_line_limit_test.rs`.
- Split files over roughly 500 lines into focused child modules.
- Keep unit tests next to their owner; shared module test helpers may live in a
  child `tests.rs`.
