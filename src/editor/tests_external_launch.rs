//! Unit tests: the two external-editor entry points — the launch
//! [`handle_edit`] performs for real, and its test-mode twin.
//!
//! `handle_edit` is the only place that builds the external editor's
//! `std::process::Command`, and it is reached through the real `CrosstermOps`.
//! Both are exercised headless here, so the command really runs — against a stub
//! that records the argv it was handed. What makes that work without a pty is
//! [`SessionGuard`]: crossterm's `disable_raw_mode` is already a no-op while raw
//! mode is off, and the guard supplies the one precondition Windows adds.

use super::tests::{build_app, select_entry};
use super::{handle_edit, handle_edit_test_mode, TestEditorBehavior};

use crate::models::Panel;

use crate::test_support::EditorEnvGuard;
use crate::WritableTestBackend;

use ratatui::Terminal;

use std::fs;
use std::io;
use std::path::Path;

use tempfile::TempDir;

/// The host-terminal state the live session has already established by the time
/// anything suspends it, set up and torn down around one test.
///
/// Mouse capture is the Windows half. There `DisableMouseCapture` goes through
/// the console API rather than an escape sequence, and crossterm refuses it with
/// "Initial console modes not set" unless capture was enabled first — so
/// `TerminalSuspension::suspend` fails on its very first step and `handle_edit`
/// returns before the editor is ever launched. The live session always has
/// capture on by then; on Unix the suspension already succeeds headless, so
/// there is nothing to set up.
///
/// Raw mode is the other half: resuming the session calls the real
/// `enable_raw_mode`, the one operation in this path that reaches the host
/// terminal. On a runner that has a tty it succeeds, so undo it however the test
/// ends rather than leaving the rest of the binary — and the developer's shell —
/// in raw mode.
struct SessionGuard;

impl SessionGuard {
    fn enter() -> Self {
        #[cfg(windows)]
        let _ = crossterm::execute!(io::stdout(), crossterm::event::EnableMouseCapture);
        Self
    }
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        #[cfg(windows)]
        let _ = crossterm::execute!(io::stdout(), crossterm::event::DisableMouseCapture);
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

/// `$EDITOR` is conventionally "program plus flags", and the file to edit is
/// appended to it. Getting either half wrong opens the wrong file or drops the
/// user's flags, so assert on the argv the launched program actually received.
#[cfg(unix)]
#[test]
fn external_editor_receives_the_editor_flags_and_then_the_selected_path() -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new()?;
    let argv_log = temp_dir.path().join("argv.txt");
    let stub = temp_dir.path().join("record-argv.sh");
    fs::write(
        &stub,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\n",
            argv_log.display()
        ),
    )?;
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755))?;

    let edited = temp_dir.path().join("notes.txt");
    fs::write(&edited, "hello")?;

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "notes.txt"));

    let _session = SessionGuard::enter();
    // "<stub> -w" is not itself a file, so it splits into program plus one flag.
    let _editor = EditorEnvGuard::set(Path::new(&format!("{} -w", stub.display())));
    let mut terminal = Terminal::new(WritableTestBackend::new(40, 8))?;

    // The resume half calls `enable_raw_mode`, which fails without a tty; the
    // launch under test has already happened by then.
    let _ = handle_edit(&mut terminal, &mut app);

    assert_eq!(
        fs::read_to_string(&argv_log)?,
        format!("-w\n{}\n", edited.display()),
        "the editor was launched with the wrong arguments"
    );
    assert_eq!(app.status_text(), format!("Edited {}", edited.display()));
    Ok(())
}

/// An editor that cannot be launched at all must land in the status bar, not in
/// the caller: `handle_edit` runs from the event loop, where an `Err` is fatal.
#[test]
fn external_editor_reports_a_launch_failure_from_the_real_command() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("notes.txt"), "hello")?;

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "notes.txt"));

    let _session = SessionGuard::enter();
    let _editor = EditorEnvGuard::set(Path::new("ffm-no-such-editor-b7f19c4e"));
    let mut terminal = Terminal::new(WritableTestBackend::new(40, 8))?;

    let _ = handle_edit(&mut terminal, &mut app);

    assert!(
        app.status_text().starts_with("Failed to launch editor:"),
        "status was {:?}",
        app.status_text()
    );
    Ok(())
}

/// Both panels are reloaded once the editor is done, and *either* reload can
/// fail. The inactive panel's failure is the one that is easy to swallow, so
/// pin it: the caller must still be told the panels are stale.
#[test]
fn test_mode_propagates_a_reload_failure_from_the_inactive_panel() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("notes.txt"), "hello")?;

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "notes.txt"));
    // Only the right (inactive) panel is broken, so the left reload and the
    // status line still succeed and the error can only come from the right one.
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel.path = temp_dir.path().join("removed-right-panel");

    let error = handle_edit_test_mode(&mut app, TestEditorBehavior::Success)
        .expect_err("a panel that cannot be reloaded must not be reported as success");

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    Ok(())
}
