# Architecture Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make runtime behavior feature-independent, enforce modal input, give operations one owner, and guarantee terminal cleanup.

**Architecture:** `ops` becomes the canonical operation layer and `handlers` becomes a compatibility facade. The event controller routes active overlays before global actions. A private, injected `TerminalOps` boundary supports RAII guards for application and external-editor terminal transitions.

**Tech Stack:** Rust 2021, crossterm 0.29, ratatui 0.30, Cargo tests, Clippy.

---

## File map

- Create `tests/module_boundaries_test.rs`: executable dependency/ownership and runtime-parity contracts.
- Create `src/ops/mod.rs`: canonical public operation facade.
- Create `src/ops/file.rs`: local file mutations and their fault-injection points.
- Create `src/ops/dispatch.rs`: typed command execution.
- Create `src/ops/shell.rs`: shell process execution.
- Create `src/ops/viewer.rs`: file viewer and MCP status viewer.
- Create `src/terminal.rs`: injected terminal operations, application RAII session, and editor suspension.
- Modify `src/handlers/mod.rs`: compatibility-only facade.
- Modify `src/popup.rs`: depend on `ops` and consume every key while modal.
- Modify `src/lib.rs`: feature-independent runtime, strict key controller, RAII session, explicit-path logging.
- Modify `src/editor.rs`: feature-independent public editor and RAII suspension.
- Modify `src/tests.rs`: deterministic logging tests; remove fake runtime tests and global `cwd` mutation.
- Modify `src/handlers/tests_popup_command_line.rs`: strict modal expectations.
- Modify `src/popup/tests.rs`: modal input regressions.
- Modify `src/editor/tests.rs`: explicit test helper where fake behavior is intended.
- Modify `tests/coverage_100_test.rs`: stop calling the real public runtime for coverage.
- Modify `tests/coverage_smoke_test.rs`: call the explicit editor test helper.
- Modify architecture documentation to match the implemented dependency direction.
- Delete `src/ops.rs` and the four implementation files under `src/handlers/` after their code and tests move.

### Task 1: Establish executable architecture contracts

**Files:**
- Create: `tests/module_boundaries_test.rs`

- [ ] **Step 1: Write the failing architecture tests**

Create the complete test file:

```rust
//! Architectural dependency and ownership contracts.

use std::fs;
use std::path::{Path, PathBuf};

fn rust_files(path: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path).expect("read source directory") {
        let path = entry.expect("read source entry").path();
        if path.is_dir() {
            rust_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
}

fn definition_owners(symbol: &str) -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    let needle = format!("fn {symbol}(");
    let mut owners = files
        .into_iter()
        .filter(|path| {
            fs::read_to_string(path)
                .expect("read Rust source")
                .contains(&needle)
        })
        .map(|path| {
            path.strip_prefix(root)
                .expect("source below manifest")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect::<Vec<_>>();
    owners.sort();
    owners
}

#[test]
fn operations_have_one_canonical_owner() {
    for (symbol, owner) in [
        ("execute_command", "src/ops/dispatch.rs"),
        ("handle_copy", "src/ops/file.rs"),
        ("handle_move", "src/ops/file.rs"),
        ("handle_mkdir", "src/ops/file.rs"),
        ("handle_delete_confirm", "src/ops/file.rs"),
        ("handle_delete", "src/ops/file.rs"),
        ("handle_create_file", "src/ops/file.rs"),
        ("rename_or_fail", "src/ops/file.rs"),
        ("run_shell_command", "src/ops/shell.rs"),
        ("handle_view", "src/ops/viewer.rs"),
        ("open_mcp_status", "src/ops/viewer.rs"),
    ] {
        assert_eq!(definition_owners(symbol), vec![owner.to_owned()], "{symbol}");
    }
}

#[test]
fn popup_does_not_depend_on_compatibility_handlers() {
    let popup = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/popup.rs"),
    )
    .expect("read popup.rs");
    assert!(!popup.contains("crate::handlers"));
}

#[test]
fn obsolete_handler_implementation_files_are_removed() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for relative in [
        "src/handlers/dispatch.rs",
        "src/handlers/file_ops.rs",
        "src/handlers/mcp.rs",
        "src/handlers/shell.rs",
    ] {
        assert!(!root.join(relative).exists(), "{relative} still exists");
    }
}

#[test]
fn handlers_keeps_stable_operation_signatures() {
    use funny_file_manager::{handlers, models::App};
    use std::io;

    let _: fn(&mut App) -> io::Result<()> = handlers::handle_copy;
    let _: fn(&mut App) -> io::Result<()> = handlers::handle_move;
    let _: fn(&mut App) -> io::Result<()> = handlers::handle_mkdir;
    let _: fn(&mut App) -> io::Result<()> = handlers::handle_delete_confirm;
    let _: fn(&mut App) -> io::Result<()> = handlers::handle_delete;
    let _: fn(&mut App) -> io::Result<()> = handlers::handle_view;
    let _: fn(&mut App) -> io::Result<()> = handlers::open_mcp_status;
    let _: fn(&mut App, &str) -> io::Result<()> = handlers::execute_command;
}

#[test]
fn test_helpers_do_not_select_run_implementation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lib = fs::read_to_string(root.join("src/lib.rs")).expect("read lib.rs");
    let run_start = lib.find("pub fn run()").expect("run entry");
    let run_end = lib.find("pub fn run_app").expect("injected event loop");
    let runtime_entry = &lib[run_start..run_end];
    assert!(!runtime_entry.contains("feature = \"test-helpers\""));
    assert_eq!(runtime_entry.matches("fn main_impl").count(), 1);
}

#[test]
fn test_helpers_do_not_select_editor_implementation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let editor = fs::read_to_string(root.join("src/editor.rs")).expect("read editor.rs");
    let editor_start = editor.find("pub fn handle_edit<").expect("editor entry");
    let helper_start = editor
        .find("pub enum TestEditorBehavior")
        .expect("explicit editor helper");
    let runtime_editor = &editor[editor_start..helper_start];
    assert!(!runtime_editor.contains("feature = \"test-helpers\""));
    assert_eq!(runtime_editor.matches("pub fn handle_edit<").count(), 1);
}
```

- [ ] **Step 2: Run the architecture test and verify RED**

Run:

```bash
cargo test --test module_boundaries_test
```

Expected: failures report duplicate/wrong owners, `popup` references
`crate::handlers`, old handler implementation files exist, and runtime source
contains `feature = "test-helpers"`.

- [ ] **Step 3: Commit only the red contract**

```bash
git add tests/module_boundaries_test.rs
git commit -m "test: define architecture boundary contracts"
```

### Task 2: Move all operation implementations under `ops`

**Files:**
- Create: `src/ops/mod.rs`
- Create: `src/ops/file.rs`
- Create: `src/ops/dispatch.rs`
- Create: `src/ops/shell.rs`
- Create: `src/ops/viewer.rs`
- Modify: `src/handlers/mod.rs`
- Modify: `src/popup.rs`
- Modify: `src/lib.rs`
- Delete: `src/ops.rs`
- Delete: `src/handlers/dispatch.rs`
- Delete: `src/handlers/file_ops.rs`
- Delete: `src/handlers/mcp.rs`
- Delete: `src/handlers/shell.rs`

- [ ] **Step 1: Create the canonical `ops` facade**

Use this complete `src/ops/mod.rs`:

```rust
//! Canonical application operations.

mod dispatch;
mod file;
mod shell;
mod viewer;

pub use dispatch::execute_command;
pub use file::{
    handle_copy, handle_delete, handle_delete_confirm, handle_mkdir, handle_move,
};
pub use viewer::{handle_view, open_mcp_status};

pub(crate) use file::handle_create_file;

#[cfg(test)]
pub(crate) use file::{
    format_operation_status, get_source_paths, rename_or_fail, FORCE_RENAME_FAILURE,
    RENAME_TEST_MUTEX,
};

#[cfg(test)]
pub(crate) use shell::{run_shell_command, windows_shell_args};
```

- [ ] **Step 2: Move implementations without changing behavior**

Apply these exact mechanical transformations:

```text
src/handlers/file_ops.rs  -> src/ops/file.rs
src/handlers/dispatch.rs  -> src/ops/dispatch.rs
src/handlers/shell.rs     -> src/ops/shell.rs
src/ops.rs                -> src/ops/viewer.rs
```

In `src/ops/file.rs`, remove the existing `handle_view` function and remove
`Overlay` and `Viewer` from its imports. Add this operation beside the other
file mutations:

```rust
pub(crate) fn handle_create_file(app: &mut App, filename: &str) -> io::Result<()> {
    let filename = filename.trim();
    if filename.is_empty() {
        app.set_status(Status::FilenameEmpty);
        return Ok(());
    }

    let file_path = app.get_active_panel().path.join(filename);
    if file_path.exists() {
        app.set_status(Status::FileExists(filename.to_owned()));
        return Ok(());
    }

    fs::File::create(file_path)?;
    app.set_status(Status::CreatedFile(filename.to_owned()));
    let _ = refresh_panels(app);
    Ok(())
}
```

In `src/ops/dispatch.rs`, use:

```rust
use super::shell::run_shell_command;
use super::viewer::open_mcp_status;
```

Remove `use super::file_ops;` and `pub use file_ops::handle_delete;`.

In `src/ops/viewer.rs`, retain only the canonical `handle_view`,
`open_mcp_status`, and their tests. The copied implementations from
`src/ops.rs` are the source of truth; do not retain the duplicate
`src/handlers/mcp.rs` implementation.

- [ ] **Step 3: Replace `handlers` with a compatibility facade**

Keep the existing `#[cfg(test)] mod tests_*` declarations and replace the
implementation-module declarations/re-exports at the top of
`src/handlers/mod.rs` with:

```rust
//! Backward-compatible facade for operations and popup input.

pub use crate::ops::{
    execute_command, handle_copy, handle_delete, handle_delete_confirm, handle_mkdir,
    handle_move, handle_view, open_mcp_status,
};
pub use crate::popup::process_popup_input;

#[doc(hidden)]
pub mod dispatch {
    pub use crate::ops::execute_command;
}

#[doc(hidden)]
pub mod file_ops {
    pub use crate::ops::{
        handle_copy, handle_delete, handle_delete_confirm, handle_mkdir, handle_move,
        handle_view,
    };

    #[cfg(test)]
    pub(crate) use crate::ops::{
        format_operation_status, get_source_paths, rename_or_fail, FORCE_RENAME_FAILURE,
        RENAME_TEST_MUTEX,
    };
}

#[doc(hidden)]
pub mod mcp {
    pub use crate::ops::open_mcp_status;
}

#[cfg(test)]
pub(crate) mod shell {
    pub(crate) use crate::ops::{run_shell_command, windows_shell_args};
}
```

- [ ] **Step 4: Point core modules directly at `ops`**

In `src/popup.rs`, replace the `std` import with:

```rust
use std::path::PathBuf;
```

and add:

```rust
use crate::ops::{execute_command, handle_create_file, handle_delete};
```

Replace the two reverse dependencies:

```rust
crate::handlers::execute_command(app, &command)
```

with:

```rust
execute_command(app, &command)
```

and:

```rust
crate::handlers::handle_delete(app)
```

with:

```rust
handle_delete(app)
```

Replace the whole `Overlay::NewFileInput` Enter body after
`app.close_overlay()`—including the popup's empty-name and existence
checks—with:

```rust
if let Err(err) = handle_create_file(app, &filename) {
    app.set_status(crate::status::Status::CreateFileFailed {
        error: err.to_string(),
    });
}
```

In `src/lib.rs`, import file operations from `crate::ops`, import
`process_popup_input` from `crate::popup`, and replace
`handlers::open_mcp_status(app)?` with `crate::ops::open_mcp_status(app)?`.

- [ ] **Step 5: Delete obsolete implementation files**

Delete exactly:

```text
src/ops.rs
src/handlers/dispatch.rs
src/handlers/file_ops.rs
src/handlers/mcp.rs
src/handlers/shell.rs
```

- [ ] **Step 6: Run focused tests and verify GREEN**

Run:

```bash
cargo test --test module_boundaries_test operations_have_one_canonical_owner
cargo test --test module_boundaries_test popup_does_not_depend_on_compatibility_handlers
cargo test --test module_boundaries_test handlers_keeps_stable_operation_signatures
cargo test --lib test_ops_view_file
cargo test --lib test_delete_confirm_overlay_y_confirms
cargo test --features test-helpers --test handlers_test
```

Expected: every command passes.

- [ ] **Step 7: Commit the operation boundary**

```bash
git add src/ops src/handlers src/popup.rs src/lib.rs tests/module_boundaries_test.rs
git commit -m "refactor: make ops the operation owner"
```

### Task 3: Enforce strict modal dispatch

**Files:**
- Modify: `src/popup.rs`
- Modify: `src/lib.rs`
- Modify: `src/handlers/tests_popup_command_line.rs`
- Modify: `src/popup/tests.rs`
- Test: existing event-loop unit tests under `src/tests_*.rs`

- [ ] **Step 1: Write failing modal tests**

Add three tests to `src/popup/tests.rs`, constructing modified keys explicitly:

```rust
#[test]
fn command_line_consumes_unbound_function_key() {
    let mut app = App::new().expect("app");
    app.set_overlay(Overlay::CommandLine("pwd".to_owned()));

    assert!(process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::F(8), KeyModifiers::empty()),
    ));
    assert!(matches!(app.overlay, Overlay::CommandLine(ref value) if value == "pwd"));
}

#[test]
fn command_line_consumes_alt_character() {
    let mut app = App::new().expect("app");
    app.set_overlay(Overlay::CommandLine("pwd".to_owned()));

    assert!(process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT),
    ));
    assert!(matches!(app.overlay, Overlay::CommandLine(ref value) if value == "pwd"));
}

#[test]
fn command_line_consumes_control_character() {
    let mut app = App::new().expect("app");
    app.set_overlay(Overlay::CommandLine("pwd".to_owned()));

    assert!(process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
    ));
    assert!(matches!(app.overlay, Overlay::CommandLine(ref value) if value == "pwd"));
}
```

Update existing command-line tests that expect unknown keys to return `false`;
they must expect `true`.

- [ ] **Step 2: Run modal tests and verify RED**

Run:

```bash
cargo test --lib command_line_consumes_
```

Expected: all three tests fail because the command-line fallback returns
`false`.

- [ ] **Step 3: Consume every key while the command line is active**

In the `Overlay::CommandLine` match in `src/popup.rs`, replace:

```rust
_ => false,
```

with:

```rust
_ => true,
```

- [ ] **Step 4: Make the controller route by modal state**

Extract this private controller in `src/lib.rs`:

```rust
fn dispatch_key<B>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    key: crossterm::event::KeyEvent,
    last_nav_time: &mut Instant,
    nav_debounce: Duration,
) -> io::Result<Flow>
where
    B: Backend + Write,
{
    if app.is_overlay_active() {
        process_popup_input(app, key);
        return if app.should_quit {
            Ok(Flow::Quit)
        } else {
            Ok(Flow::Continue)
        };
    }

    apply_action(
        terminal,
        app,
        resolve(key),
        last_nav_time,
        nav_debounce,
    )
}
```

Replace the popup/apply block in `run_app_with_event_reader` with one
`dispatch_key` call and handle `Flow::Quit` by returning `Ok(())`.

- [ ] **Step 5: Verify modal and existing input behavior**

Run:

```bash
cargo test --lib command_line_consumes_
cargo test --lib tests_popup_command_line
cargo test --lib tests_event_loop
```

Expected: all selected tests pass.

- [ ] **Step 6: Commit strict modal routing**

```bash
git add src/lib.rs src/popup.rs src/popup/tests.rs src/handlers/tests_popup_command_line.rs
git commit -m "fix: keep global actions behind modal overlays"
```

### Task 4: Remove feature-selected runtime behavior and the `cwd` race

**Files:**
- Modify: `src/lib.rs`
- Modify: `src/tests.rs`
- Modify: `src/tests_event_loop_keys.rs`
- Modify: `tests/coverage_100_test.rs`
- Test: `tests/module_boundaries_test.rs`

- [ ] **Step 1: Rewrite logging tests against explicit paths**

Replace the logging lifecycle tests in `src/tests.rs` with:

```rust
#[test]
fn test_run_with_logs_records_error_at_explicit_paths() -> io::Result<()> {
    let temp = TempDir::new()?;
    let actions = temp.path().join("actions.log");
    let errors = temp.path().join("errors.log");

    let result = run_with_logs(
        || Err(io::Error::other("boom").into()),
        &actions,
        &errors,
    );

    assert!(result.is_err());
    assert!(fs::read_to_string(actions)?.contains("Started"));
    assert!(fs::read_to_string(errors)?.contains("Main error: boom"));
    Ok(())
}

#[test]
fn test_run_with_logs_records_success_at_explicit_paths() -> io::Result<()> {
    let temp = TempDir::new()?;
    let actions = temp.path().join("actions.log");
    let errors = temp.path().join("errors.log");

    run_with_logs(|| Ok(()), &actions, &errors)?;

    let contents = fs::read_to_string(actions)?;
    assert!(contents.contains("Started"));
    assert!(contents.contains("Ended"));
    assert!(!errors.exists());
    Ok(())
}

#[test]
fn test_log_writers_ignore_unwritable_explicit_paths() -> io::Result<()> {
    let temp = TempDir::new()?;
    let actions = temp.path().join("actions");
    let errors = temp.path().join("errors");
    fs::create_dir(&actions)?;
    fs::create_dir(&errors)?;

    log_action_to(&actions, "action");
    log_error_to(&errors, "error");

    assert!(actions.is_dir());
    assert!(errors.is_dir());
    Ok(())
}
```

Delete `cwd_mutex`, `CwdGuard`, `test_main_impl_test_mode`, and tests in
`src/tests.rs` and `src/tests_event_loop_keys.rs` that call the public `run()`
only for coverage.

- [ ] **Step 2: Run the new logging test and verify RED**

Run:

```bash
cargo test --lib test_run_with_logs_records_error_at_explicit_paths
```

Expected: compilation fails because `run_with_logs`, `log_action_to`, and
`log_error_to` do not exist.

- [ ] **Step 3: Add explicit-path logging**

Use these functions in `src/lib.rs`:

```rust
const ACTION_LOG: &str = "ffm_actions.log";
const ERROR_LOG: &str = "ffm_errors.log";

fn append_log(path: &std::path::Path, message: &str) {
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_millis());
        let _ = writeln!(file, "[{timestamp}] {message}");
    }
}

fn log_action_to(path: &std::path::Path, action: &str) {
    append_log(path, action);
}

fn log_error_to(path: &std::path::Path, error: &str) {
    append_log(path, &format!("ERROR: {error}"));
}

fn log_action(action: &str) {
    log_action_to(std::path::Path::new(ACTION_LOG), action);
}

fn log_error(error: &str) {
    log_error_to(std::path::Path::new(ERROR_LOG), error);
}

fn run_with_logs<F>(
    main_runner: F,
    action_log: &std::path::Path,
    error_log: &std::path::Path,
) -> Result<(), Box<dyn Error>>
where
    F: FnOnce() -> Result<(), Box<dyn Error>>,
{
    log_action_to(action_log, "=== FunnyFileManager Started ===");
    let result = main_runner();
    if let Err(ref error) = result {
        log_error_to(error_log, &format!("Main error: {error}"));
    }
    log_action_to(action_log, "=== FunnyFileManager Ended ===");
    result
}

fn run_with<F>(main_runner: F) -> Result<(), Box<dyn Error>>
where
    F: FnOnce() -> Result<(), Box<dyn Error>>,
{
    run_with_logs(
        main_runner,
        std::path::Path::new(ACTION_LOG),
        std::path::Path::new(ERROR_LOG),
    )
}
```

- [ ] **Step 4: Give runtime entry points one implementation**

In `src/lib.rs`:

- import real crossterm events and `CrosstermBackend` unconditionally;
- delete the fake `event` module;
- make `run()` exactly:

```rust
pub fn run() -> Result<(), Box<dyn Error>> {
    run_with(main_impl)
}
```

- retain exactly one `fn main_impl() -> Result<(), Box<dyn Error>>`;
- keep `test_support` and `WritableTestBackend` exposure under
  `cfg(any(test, feature = "test-helpers"))`.

Remove the two `tests/coverage_100_test.rs` tests that invoke `run()` and the
unit tests that invoke `run()`/fake `main_impl`; the injected event-loop tests
already cover the testable runtime logic.

- [ ] **Step 5: Verify runtime parity and deterministic tests**

Run:

```bash
cargo test --test module_boundaries_test test_helpers_do_not_select_run_implementation
cargo test --lib test_run_with_logs_
cargo test
cargo check --all-features --bins
```

Expected: every command passes; repeated `cargo test` no longer races on the
logging tests' current directory.

- [ ] **Step 6: Commit runtime parity**

```bash
git add src/lib.rs src/tests.rs src/tests_event_loop_keys.rs tests/coverage_100_test.rs tests/module_boundaries_test.rs
git commit -m "fix: keep test helpers out of runtime semantics"
```

### Task 5: Add RAII terminal sessions for main and external editor

**Files:**
- Create: `src/terminal.rs`
- Modify: `src/lib.rs`
- Modify: `src/editor.rs`
- Modify: `src/editor/tests.rs`
- Modify: `tests/coverage_smoke_test.rs`

- [ ] **Step 1: Add failing tests for the wished-for terminal API**

Add `mod terminal;` to `src/lib.rs`. Create `src/terminal.rs` with this test
module first; it intentionally references the wished-for API before that API
exists:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::WritableTestBackend;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Clone)]
    struct RecordingOps {
        calls: Rc<RefCell<Vec<&'static str>>>,
        fail_on: Option<&'static str>,
    }

    impl RecordingOps {
        fn new(fail_on: Option<&'static str>) -> Self {
            Self {
                calls: Rc::new(RefCell::new(Vec::new())),
                fail_on,
            }
        }

        fn record(&self, operation: &'static str) -> io::Result<()> {
            self.calls.borrow_mut().push(operation);
            if self.fail_on == Some(operation) {
                Err(io::Error::other(operation))
            } else {
                Ok(())
            }
        }
    }

    impl TerminalOps<WritableTestBackend> for RecordingOps {
        fn enable_raw(&mut self) -> io::Result<()> {
            self.record("enable_raw")
        }

        fn disable_raw(&mut self) -> io::Result<()> {
            self.record("disable_raw")
        }

        fn enter_alternate(
            &mut self,
            _terminal: &mut Terminal<WritableTestBackend>,
        ) -> io::Result<()> {
            self.record("enter_alternate")
        }

        fn leave_alternate(
            &mut self,
            _terminal: &mut Terminal<WritableTestBackend>,
        ) -> io::Result<()> {
            self.record("leave_alternate")
        }

        fn enable_mouse(
            &mut self,
            _terminal: &mut Terminal<WritableTestBackend>,
        ) -> io::Result<()> {
            self.record("enable_mouse")
        }

        fn disable_mouse(
            &mut self,
            _terminal: &mut Terminal<WritableTestBackend>,
        ) -> io::Result<()> {
            self.record("disable_mouse")
        }

        fn clear(
            &mut self,
            _terminal: &mut Terminal<WritableTestBackend>,
        ) -> io::Result<()> {
            self.record("clear")
        }

        fn show_cursor(
            &mut self,
            _terminal: &mut Terminal<WritableTestBackend>,
        ) -> io::Result<()> {
            self.record("show_cursor")
        }
    }

    fn terminal() -> Terminal<WritableTestBackend> {
        Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal")
    }

#[test]
fn session_drop_restores_in_reverse_order() {
    let mut terminal = terminal();
    let mut ops = RecordingOps::new(None);
    let calls = Rc::clone(&ops.calls);
    {
        let _session =
            TerminalSession::enter(&mut terminal, &mut ops).expect("enter session");
    }

    assert_eq!(
        *calls.borrow(),
        [
            "enable_raw",
            "enter_alternate",
            "enable_mouse",
            "show_cursor",
            "disable_mouse",
            "leave_alternate",
            "disable_raw",
        ]
    );
}

#[test]
fn partial_enter_is_rolled_back() {
    let mut terminal = terminal();
    let mut ops = RecordingOps::new(Some("enter_alternate"));
    let calls = Rc::clone(&ops.calls);
    assert!(TerminalSession::enter(&mut terminal, &mut ops).is_err());

    assert_eq!(
        *calls.borrow(),
        ["enable_raw", "enter_alternate", "show_cursor", "disable_raw"]
    );
}

#[test]
fn cleanup_continues_after_failure() {
    let mut terminal = terminal();
    let mut ops = RecordingOps::new(Some("disable_mouse"));
    let calls = Rc::clone(&ops.calls);
    let session = TerminalSession::enter(&mut terminal, &mut ops).expect("enter");
    assert!(session.finish().is_err());

    assert!(calls.borrow().windows(4).any(|window| window == [
        "show_cursor",
        "disable_mouse",
        "leave_alternate",
        "disable_raw",
    ]));
}

#[test]
fn run_error_has_priority_over_cleanup_error() {
    let run = Err::<(), _>(io::Error::other("run"));
    let cleanup = Err(io::Error::other("cleanup"));
    assert_eq!(
        merge_run_and_cleanup(run, cleanup)
            .expect_err("primary error")
            .to_string(),
        "run"
    );
}

#[test]
fn suspension_resumes_in_forward_order() {
    let mut terminal = terminal();
    let mut ops = RecordingOps::new(None);
    let calls = Rc::clone(&ops.calls);
    let suspension =
        TerminalSuspension::suspend(&mut terminal, &mut ops).expect("suspend");
    suspension.resume().expect("resume");

    assert_eq!(
        *calls.borrow(),
        [
            "disable_mouse",
            "leave_alternate",
            "disable_raw",
            "enable_raw",
            "enter_alternate",
            "enable_mouse",
            "clear",
        ]
    );
}

#[test]
fn suspension_resume_continues_after_failure() {
    let mut terminal = terminal();
    let mut ops = RecordingOps::new(Some("enter_alternate"));
    let calls = Rc::clone(&ops.calls);
    let suspension =
        TerminalSuspension::suspend(&mut terminal, &mut ops).expect("suspend");
    assert!(suspension.resume().is_err());

    assert!(calls.borrow().windows(4).any(|window| window == [
        "enable_raw",
        "enter_alternate",
        "enable_mouse",
        "clear",
    ]));
}
}
```

- [ ] **Step 2: Run the terminal tests and verify RED**

Run:

```bash
cargo test --lib terminal::tests
```

Expected: compilation fails because `TerminalOps`, `TerminalSession`,
`TerminalSuspension`, and `merge_run_and_cleanup` are not defined.

- [ ] **Step 3: Implement the minimal terminal boundary**

Implement these private signatures in `src/terminal.rs`:

```rust
pub(crate) trait TerminalOps<B>
where
    B: Backend + Write,
{
    fn enable_raw(&mut self) -> io::Result<()>;
    fn disable_raw(&mut self) -> io::Result<()>;
    fn enter_alternate(&mut self, terminal: &mut Terminal<B>) -> io::Result<()>;
    fn leave_alternate(&mut self, terminal: &mut Terminal<B>) -> io::Result<()>;
    fn enable_mouse(&mut self, terminal: &mut Terminal<B>) -> io::Result<()>;
    fn disable_mouse(&mut self, terminal: &mut Terminal<B>) -> io::Result<()>;
    fn clear(&mut self, terminal: &mut Terminal<B>) -> io::Result<()>;
    fn show_cursor(&mut self, terminal: &mut Terminal<B>) -> io::Result<()>;
}

pub(crate) struct CrosstermOps;

#[cfg(any(test, coverage))]
pub(crate) struct NoopTerminalOps;

pub(crate) struct TerminalSession<'a, B, O>
where
    B: Backend + Write,
    O: TerminalOps<B>,
{
    terminal: &'a mut Terminal<B>,
    ops: &'a mut O,
    raw: bool,
    alternate: bool,
    mouse: bool,
    cursor_pending: bool,
}

impl<'a, B, O> TerminalSession<'a, B, O>
where
    B: Backend + Write,
    O: TerminalOps<B>,
{
    pub(crate) fn enter(
        terminal: &'a mut Terminal<B>,
        ops: &'a mut O,
    ) -> io::Result<Self>;
    pub(crate) fn terminal_mut(&mut self) -> &mut Terminal<B>;
    pub(crate) fn finish(self) -> io::Result<()>;
}

pub(crate) struct TerminalSuspension<'a, B, O>
where
    B: Backend + Write,
    O: TerminalOps<B>,
{
    terminal: &'a mut Terminal<B>,
    ops: &'a mut O,
    raw_disabled: bool,
    alternate_left: bool,
    mouse_disabled: bool,
}

impl<'a, B, O> TerminalSuspension<'a, B, O>
where
    B: Backend + Write,
    O: TerminalOps<B>,
{
    pub(crate) fn suspend(
        terminal: &'a mut Terminal<B>,
        ops: &'a mut O,
    ) -> io::Result<Self>;
    pub(crate) fn resume(self) -> io::Result<()>;
}

pub(crate) fn merge_run_and_cleanup<T>(
    run: io::Result<T>,
    cleanup: io::Result<()>,
) -> io::Result<T>;
```

Both guards must track each successful transition, attempt every applicable
reverse transition while retaining the first cleanup error, clear a flag only
after a successful reversal, and call the same restore/resume method from
`Drop` on unfinished guards.

`CrosstermOps` maps the trait methods directly to crossterm commands. Each
mouse/alternate command operates on `terminal.backend_mut()`; `clear` and
`show_cursor` use the corresponding `Terminal` methods.

`NoopTerminalOps` returns `Ok(())` from every method. It is available only to
crate unit/coverage tests and never selected by the public runtime.

- [ ] **Step 4: Use `TerminalSession` in `main_impl`**

Construct `App` and `Terminal` before entering terminal mode. Then use:

```rust
let mut terminal_ops = crate::terminal::CrosstermOps;
let mut session =
    crate::terminal::TerminalSession::enter(&mut terminal, &mut terminal_ops)?;
let run_result = run_app(session.terminal_mut(), app);
let cleanup_result = session.finish();
crate::terminal::merge_run_and_cleanup(run_result, cleanup_result)
    .map_err(std::convert::Into::into)
```

Remove direct raw-mode and alternate-screen setup/cleanup from `lib.rs`.

- [ ] **Step 5: Use `TerminalSuspension` in external editing**

First add one core function that accepts explicit terminal operations and an
editor runner:

```rust
fn handle_edit_with<B, F, O>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    terminal_ops: &mut O,
    run_editor: F,
) -> io::Result<()>
where
    B: Backend + Write,
    O: crate::terminal::TerminalOps<B>,
    F: FnOnce(&str, &[String], &std::path::Path) -> io::Result<std::process::ExitStatus>,
```

Move the existing selection, editor-choice, runner-status, and panel-refresh
logic into this function. Remove direct crossterm suspend/resume commands.
Suspend before `run_editor` and resume immediately after it:

```rust
let suspension =
    crate::terminal::TerminalSuspension::suspend(terminal, terminal_ops)?;
let status = run_editor(&editor, &args, &selected);
let resume_result = suspension.resume();
```

Keep the existing status mapping for editor success, non-zero exit, and launch
failure. After refreshing panels, return `resume_result` so a terminal restore
failure is visible to the controller.

Expose exactly one public runtime implementation:

```rust
pub fn handle_edit<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    let mut terminal_ops = crate::terminal::CrosstermOps;
    handle_edit_with(terminal, app, &mut terminal_ops, |editor, args, path| {
        std::process::Command::new(editor)
            .args(args)
            .arg(path)
            .status()
    })
}
```

Preserve the existing unit-test call sites with an explicit crate-only wrapper:

```rust
#[cfg(any(test, coverage))]
pub(crate) fn handle_edit_impl<B, F>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    run_editor: F,
) -> io::Result<()>
where
    B: Backend + Write,
    F: FnOnce(&str, &[String], &std::path::Path) -> io::Result<std::process::ExitStatus>,
{
    let mut terminal_ops = crate::terminal::NoopTerminalOps;
    handle_edit_with(terminal, app, &mut terminal_ops, run_editor)
}
```

Retain `TestEditorBehavior` and `handle_edit_test_mode` behind
`cfg(any(test, coverage, feature = "test-helpers"))`; these are explicit
helpers, not alternate definitions of `handle_edit`.

Update `test_handle_edit_public_is_safe_in_tests` in `src/editor/tests.rs` to
call:

```rust
handle_edit_test_mode(&mut app, TestEditorBehavior::Success)?;
```

In `tests/coverage_smoke_test.rs`, make the same explicit helper call and
import `TestEditorBehavior`.

- [ ] **Step 6: Run focused and broad tests**

Run:

```bash
cargo test --lib terminal::tests
cargo test --lib editor::tests
cargo test --lib tests_event_loop
cargo test --test module_boundaries_test test_helpers_do_not_select_editor_implementation
cargo check --all-features --bins
```

Expected: all commands pass without warnings.

- [ ] **Step 7: Commit terminal lifecycle**

```bash
git add src/terminal.rs src/lib.rs src/editor.rs src/editor/tests.rs tests/coverage_smoke_test.rs tests/module_boundaries_test.rs
git commit -m "fix: restore terminal state with RAII guards"
```

### Task 6: Synchronize architecture documentation and run the quality gate

**Files:**
- Modify: `docs/specifications/command-popup-and-operations-modules.md`
- Modify: `docs/knowledge-base/02-architecture/module-structure.md`
- Modify: `docs/knowledge-base/07-core-features/file-operations.md`
- Modify: `docs/knowledge-base/07-core-features/command-line.md`
- Modify: `AGENTS.md`
- Modify: `GEMINI.md`

- [ ] **Step 1: Update documentation to the implemented truth**

Document these exact invariants:

```text
ops owns command, file, shell, viewer, and MCP operations.
popup may depend on ops but never on the handlers compatibility facade.
handlers contains compatibility re-exports and no operation implementations.
test-helpers exposes explicit seams and never changes run/handle_edit semantics.
an active Overlay consumes the key before Action::resolve.
terminal state transitions are owned by RAII guards in src/terminal.rs.
```

Update module trees and references from `src/ops.rs` and
`src/handlers/file_ops.rs` to `src/ops/{mod,file,dispatch,shell,viewer}.rs`.

- [ ] **Step 2: Run formatting**

Run:

```bash
cargo fmt --all
cargo fmt --all -- --check
```

Expected: the check exits successfully.

- [ ] **Step 3: Run lint/build gates**

Run:

```bash
cargo clippy --all-targets --all-features -- -D warnings
cargo build --all-features
```

Expected: both commands exit successfully with no warnings.

- [ ] **Step 4: Run test gates**

Run:

```bash
cargo test
cargo test --features test-helpers
cargo test --all-features
```

Expected: all test binaries pass; no baseline `cwd` race recurs.

- [ ] **Step 5: Verify the diff and commit documentation**

Run:

```bash
git diff --check
git status --short
```

Expected: no whitespace errors and only planned files are changed. Then:

```bash
git add AGENTS.md GEMINI.md docs
git commit -m "docs: describe architecture foundation"
```
