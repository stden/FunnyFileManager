# 🏛️ Architecture Foundation Design

> Status: implemented. The living description is `docs/knowledge-base/02-architecture/` and `docs/specifications/command-popup-and-operations-modules.md`. The "before this refactor" section below is history.

## 🧐 Context

Before this refactor, the event loop, modal input, and operation modules worked,
but four boundaries were unsafe:

1. `test-helpers` selected fake runtime and editor implementations. Therefore a
   binary built with `--all-features` was not the production application.
2. terminal restoration happened only after successful setup and application
   initialization. An early `?` can leave raw mode or the alternate screen
   active.
3. `handlers` and `popup` depended on each other, while `handle_view` and
   `open_mcp_status` had duplicate implementations in `handlers` and `ops`.
4. `Overlay::CommandLine` returned unconsumed input for global hotkeys, so a
   modal command line could accidentally trigger panel operations.

The baseline test suite also contained a related race: logging tests changed the
process-wide current directory to a temporary directory while
`test_main_impl_test_mode` called `App::new()`. Another thread could delete that
directory between `current_dir()` and `read_dir()`.

## 🎯 Goal

Create a small, testable foundation with:

- one owner for application operations;
- one-way module dependencies;
- strictly modal overlay input;
- runtime semantics that are independent of the `test-helpers` feature;
- best-effort terminal restoration attempts on success, error, and unwinding;
- tests that do not mutate process-wide `cwd` merely to redirect log files.

This is a compatibility-focused refactor. Existing top-level
`funny_file_manager::handlers::*` imports remain valid.

## 🚫 Non-goals

- redesigning `Panel`, `DirEntry`, bookmarks, or quick-search domain types;
- making every popup transition pure;
- implementing an S3-backed panel;
- changing copy/move transactional semantics;
- splitting the standalone MCP server.

Those are separate deliverables because they affect different domain models
and require independent migration plans.

## 📚 Dependency direction

Before:

```text
lib ──> handlers ──> popup
          ^           │
          └───────────┘

handlers/{file_ops,dispatch,mcp,shell}
ops.rs
  └─ duplicated handle_view/open_mcp_status
```

After:

```text
lib ───────────────> ops
 │                    ▲
 └─> popup ───────────┘

handlers (compatibility facade) ──> ops
                                └─> popup
```

`ops` becomes a directory module:

```text
src/ops/
├── mod.rs       public operation facade
├── dispatch.rs  typed command execution
├── file.rs      mutating local-file operations
├── shell.rs     shell process execution
└── viewer.rs    viewer-producing operations
```

The old `src/handlers/{dispatch,file_ops,mcp,shell}.rs` implementation files
are removed. `src/handlers/mod.rs` contains only compatibility re-exports,
including hidden inline submodule facades needed by existing internal tests.
`popup` calls canonical operations through `crate::ops`, never through the
compatibility facade.

This applies DRY to semantic duplicates and KISS to dependency inversion: no
service container, callback registry, or action bus is introduced.

## 🪟 Modal input contract

`process_popup_input(app, key) -> bool` keeps its public signature:

- `false` means and only means `Overlay::None`;
- every key is consumed when any overlay is active;
- unsupported command-line keys leave the command buffer unchanged and return
  `true`.

Command execution and confirmed deletion continue to happen synchronously, but
through `ops`. Moving all popup I/O into effects is deliberately deferred.

## 🧑‍💻 Runtime and test parity

`run`, `main_impl`, `crossterm::event::read`, and the external-editor entry
point have one production implementation. Enabling `test-helpers` only exposes
test utilities such as `WritableTestBackend` and explicit editor test helpers;
it does not choose a fake application.

Unit-only and coverage-only helpers may remain behind `cfg(test)` or
`cfg(coverage)`. Integration tests must call explicit seams
(`run_app_with_event_reader`, `handle_edit_test_mode`) instead of relying on a
feature that silently replaces runtime behavior.

## 🖥️ Terminal lifecycle

`src/terminal.rs` owns injected terminal operations and two RAII guards.
`TerminalOps` exposes the individual transitions; production uses
`CrosstermOps`, while unit/coverage tests use `NoopTerminalOps` or a recording
implementation.

```text
application:
Inactive --TerminalSession::enter--> Active --finish/Drop--> Inactive

external editor:
Active --TerminalSuspension::suspend--> Suspended --resume/Drop--> Active
```

`TerminalSession` enters raw mode, alternate screen, and mouse capture. Cleanup
attempts the inverse steps in reverse order, continues after a failed cleanup
operation, and restores the cursor last. `TerminalSuspension` disables mouse
capture, leaves the alternate screen, and disables raw mode; resumption
re-enables raw mode, alternate screen, and mouse capture before clearing the
TUI.

Before calling a transition that may have taken effect and then returned an
error, each guard conservatively records its inverse as pending. Consequently,
an error during partial setup or suspension triggers best-effort rollback from
`Drop`, including the inverse of the ambiguous failed operation. Explicit
cleanup retains the first cleanup error while still attempting all later
steps. `merge_run_and_cleanup` preserves the primary application/refresh error
when cleanup also fails.

## 📜 Logging and the baseline race

Logging is split into:

- production wrappers that use `ffm_actions.log` and `ffm_errors.log`;
- path-accepting internal functions used by tests.

Lifecycle logging tests pass paths inside their own `TempDir` and no longer
call `set_current_dir()`. The fake `main_impl` and tests that invoke it are
removed with the runtime-parity change. This fixes the race at its source
instead of serializing an otherwise unnecessary global mutation.

## 🚨 Error handling

Fatal runtime errors still propagate from terminal draw/input and application
startup. Existing operation return types and status behavior remain unchanged
in this refactor. A separate controller-error-policy change can later convert
recoverable file-operation errors into typed status updates without mixing that
behavioral change into the module move.

## 🔎 Verification

The implementation is test-driven:

- architecture tests first fail on duplicate owners, old implementation files,
  `popup -> handlers`, and feature-selected runtime code;
- a modal regression first demonstrates that `F8` is not consumed by an open
  command line;
- terminal-session tests first describe cleanup on drop and explicit finish;
- logging tests are rewritten to use explicit paths before global `cwd`
  mutation is removed.

Final gates:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo build --all-features
cargo test
cargo test --features test-helpers
```
