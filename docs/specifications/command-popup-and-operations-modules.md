<!-- Purpose: Specifies the command, popup, operations, and compatibility-facade boundaries. -->

# 🏛️ Command, Popup, and Operations Modules Specification

## 📖 Purpose

Define the ownership and dependency boundaries between `command`, `popup`,
`ops`, and the backward-compatible `handlers` facade so the module split stays
maintainable and testable.

## 🎯 Scope

- Command representation and parsing (`src/command.rs`)
- Modal input handling (`src/popup.rs`)
- Canonical operations (`src/ops/{mod,dispatch,file,shell,viewer}.rs`)
- Compatibility re-exports (`src/handlers/mod.rs`)
- Dispatch from the event loop in `src/lib.rs`

Rendering, the full `App`/`Overlay` model, and low-level utilities remain in
`ui`, `models`, and `utils`, respectively.

## 📚 Dependency Direction

```text
lib ───────────────> ops
 │                    ▲
 └─> popup ───────────┘

handlers (compatibility facade) ──> ops
                                └─> popup
```

`popup` may call `ops`; it must never call `handlers`. `handlers` contains no
operation implementation.

## 🏛️ Command Module (`src/command.rs`)

- `Command` is the typed representation of the `:` command language.
- `Command::parse(input: &str) -> Command` is the single source of truth for
  syntax and validation.
- Known commands become dedicated variants. Rejected recognized forms (for
  example bare `cd`, an unknown sort mode, or an invalid bookmark slot) become
  `Invalid` with command-specific usage; unknown verbs become `Shell`.
- Command completion and parsing share the same verb vocabulary. Path
  normalization used by `cd`, including leading-tilde expansion, is likewise
  shared rather than duplicated.
- Parsing is pure and independently unit-testable. Execution belongs to
  `src/ops/dispatch.rs`.

## 🏛️ Popup Module (`src/popup.rs`)

- `process_popup_input(app, key) -> bool` returns `false` only for
  `Overlay::None`.
- Every key is consumed while any overlay is active. In particular, an
  unsupported `CommandLine` key returns `true` and leaves its buffer unchanged.
- `src/lib.rs` checks for an active overlay before calling `Action::resolve`, so
  modal input cannot trigger a main-view action.
- `Overlay::CommandLine(CommandLine)` keeps the editable buffer and fuzzy
  completion menu together. Tab accepts a sole match or opens a dropdown for
  multiple matches. While it is open, text edits refresh candidates, Tab and
  Up/Down navigate, Enter accepts its selection before submitting, and Esc
  dismisses the menu before closing the overlay.
- Multiple completion candidates remain in the dropdown; popup handling does
  not publish an ambiguity state through application status.
- Command execution, file creation, and confirmed deletion call canonical
  functions from `ops`.
- Popup handling still performs some synchronous I/O for transitions such as
  drive selection and editor save/refresh. Extracting all effects from `popup`
  is not part of this architecture foundation.
- Every `Overlay` variant must be reachable from a key or `:` command and have
  an intentional presentation in `src/ui.rs` or `src/ui/overlays.rs`. Quick
  search and bookmarks intentionally are not overlay variants.

## 🏛️ Operations Module (`src/ops/`)

`ops` is the only implementation owner for command, file, shell, viewer, and
MCP-status operations:

- `mod.rs` — public operation facade and internal re-exports
- `dispatch.rs` — typed `Command` execution
- `file.rs` — create, copy, move, mkdir, and delete operations
- `shell.rs` — platform shell process execution
- `viewer.rs` — file viewer and MCP status viewer operations

New command, file, shell, viewer, or MCP-status behavior belongs in the
corresponding `ops` child module, not in `handlers`.

Shell fallback is bounded to ten seconds, accepts Esc or Ctrl+C cancellation,
and discards other keys received while the event loop is waiting for the child.

Viewer-producing operations in `ops/viewer.rs` may install
`Overlay::Viewer` and update status. `handle_view` enforces the 10 MiB limit,
rejects NUL-containing binary files, and decodes non-UTF-8 text lossily.
`open_mcp_status` reports the known IDE MCP configuration locations by scanning
for the `ffm-mcp` marker.

## 🧑‍💻 Handlers Compatibility Facade (`src/handlers/mod.rs`)

- Re-exports the established operation functions from `ops` and
  `process_popup_input` from `popup`.
- Keeps hidden compatibility submodule paths used by existing internal tests.
- Contains historical test modules, but no production operation
  implementation.
- Exists to preserve callers such as `funny_file_manager::handlers::handle_copy`
  while direct internal callers use the canonical modules.

## 🧪 Runtime and Test Boundaries

- `run`, `main_impl`, and external-editor `handle_edit` have one implementation
  regardless of whether `test-helpers` is enabled.
- `test-helpers` only exposes explicit seams such as `WritableTestBackend`,
  injected event readers, and `handle_edit_test_mode`.
- Unit/coverage helpers may use `cfg(test)` or `cfg(coverage)`; they must not
  silently replace production runtime behavior.
- Logging tests pass explicit paths to the path-accepting logging seam and do
  not change the process-wide current directory.
- Runtime logging is disabled unless `FFM_LOG=1`; enabled logs live in the
  per-user temporary directory, and character key payloads are redacted.

## 🖥️ Terminal Lifecycle

`src/terminal.rs` owns terminal transitions through `TerminalOps`,
`CrosstermOps`, `TerminalSession`, and `TerminalSuspension`. Guards perform
best-effort rollback after partial transitions, continue cleanup after an
individual failure, and restore the cursor last for the application session.

## 🚨 Fault Injection

File-operation fault controls are cfg-gated, in-memory test state—not
environment variables:

- `FORCE_RENAME_FAILURE` in `src/ops/file.rs`
- `FORCE_COPY_DIR_ALL_STEP` and `FORCE_REFRESH_ERROR` in `src/utils.rs`

The utility controls are changed through test-only setters and serialized where
shared state could otherwise make tests race.

## 🚨 Error Handling

- Operation entry points return `io::Result<()>`.
- Operations set typed or human-readable status when that is part of their
  contract; popup handling and the main-view `report` boundary translate
  routine operation errors into status rather than terminating the event loop.
- Refresh behavior is operation-specific. Code and documentation must not
  claim transactional rollback or guaranteed refresh after a partial failure.
- Copy, move, and delete continue marked-item batches after individual
  failures, report aggregate status, and refresh after the batch.
- Production operation code must not panic or unwrap user-controlled data.

## 🚫 Non-Goals

- Moving all popup I/O into a pure effect layer
- Changing copy/move transactional semantics
- Redesigning `Panel`, `DirEntry`, bookmarks, or quick search
- Implementing an S3-backed panel or restructuring the standalone MCP server

## 📄 Related Documents

- `docs/knowledge-base/02-architecture/module-structure.md`
- `docs/knowledge-base/06-ui-and-interaction/overlays-and-modals.md`
- `docs/knowledge-base/07-core-features/command-line.md`
- `docs/knowledge-base/07-core-features/file-operations.md`
