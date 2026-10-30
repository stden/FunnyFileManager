//! Unit tests: the external launcher's guards — no editor while the terminal
//! cannot be handed over — and the `$EDITOR` argument plumbing.

use super::tests::{build_app, select_entry};
use super::{handle_edit_impl, handle_edit_with};

use crate::terminal::TerminalOps;
use crate::test_support::EditorEnvGuard;
use crate::WritableTestBackend;

use ratatui::Terminal;

use std::io;
use std::path::Path;

use tempfile::TempDir;

/// Fails while suspending the terminal, before raw mode is left, so the
/// editor is never reached.
struct FailingSuspendOps;

impl TerminalOps<WritableTestBackend> for FailingSuspendOps {
    fn enable_raw(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn disable_raw(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn enter_alternate(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }

    fn leave_alternate(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }

    fn enable_mouse(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }

    fn disable_mouse(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Err(io::Error::other("disable_mouse"))
    }

    fn clear(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }

    fn show_cursor(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }

    fn wait_for_key(
        &mut self,
        _terminal: &mut Terminal<WritableTestBackend>,
        _prompt: &str,
    ) -> io::Result<()> {
        Ok(())
    }
}

/// If the terminal cannot be handed over, the editor must not run at all — a
/// second program drawing over a live TUI is worse than a refused edit.
#[test]
fn external_editor_is_not_launched_when_the_terminal_cannot_be_suspended() {
    let temp_dir = TempDir::new().expect("temp dir");
    std::fs::write(temp_dir.path().join("file.txt"), b"content").expect("write file");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut terminal_ops = FailingSuspendOps;

    let error = handle_edit_with(&mut terminal, &mut app, &mut terminal_ops, |_, _, _| {
        panic!("the editor must not run while the terminal is still in raw mode")
    })
    .expect_err("a failed suspend must be reported");

    assert_eq!(error.to_string(), "disable_mouse");
    assert_eq!(app.status_text().len(), 0);
}

/// `EDITOR="code -w -n"` must reach the launcher as a program plus two
/// arguments — those arguments are what make VS Code block until the file is
/// closed, so dropping them silently breaks the external editor.
#[test]
fn external_editor_forwards_arguments_from_the_editor_variable() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    std::fs::write(&file_path, b"content").expect("write file");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));

    // A name that is not an existing file, so the whitespace split applies.
    let _guard = EditorEnvGuard::set(Path::new("ffm-fake-code -w -n"));

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    let mut launched: Option<(String, Vec<String>)> = None;
    handle_edit_impl(&mut terminal, &mut app, |editor, args, path| {
        launched = Some((editor.to_owned(), args.to_vec()));
        assert_eq!(path, file_path.as_path());
        Ok(std::process::ExitStatus::default())
    })?;

    let (editor, args) = launched.expect("the editor runner should have been called");
    assert_eq!(editor, "ffm-fake-code");
    assert_eq!(args, ["-w", "-n"]);
    Ok(())
}
