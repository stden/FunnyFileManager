//! Unit tests: the guards F4 runs before it opens a file, plus the `$EDITOR`
//! argument plumbing that reaches the external launcher.

use super::tests::{build_app, select_entry};
use super::{
    handle_edit_builtin, handle_edit_impl, handle_edit_test_mode, handle_edit_with,
    TestEditorBehavior,
};
use crate::models::EntryKind;
use crate::models::FileSize;

use crate::models::DirEntry;
use crate::terminal::TerminalOps;
use crate::test_support::EditorEnvGuard;
use crate::utils::MAX_FILE_SIZE;
use crate::WritableTestBackend;

use ratatui::Terminal;

use std::fs::File;
use std::io;
use std::path::Path;

use tempfile::TempDir;

/// Sparse file: `set_len` records the length in the inode without writing a
/// byte, so the over-the-limit case costs no disk I/O.
fn sparse_file(dir: &Path, name: &str, len: u64) -> io::Result<()> {
    File::create(dir.join(name))?.set_len(len)
}

/// The parent entry has no path of its own. `is_dir: false` is what gets it
/// past the directory guard, so `selected_path()` is the guard under test.
fn select_parent_entry(app: &mut crate::models::App) {
    app.left_panel.entries = vec![DirEntry {
        name: "..".to_string(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(0),
    }];
    app.left_panel.select(Some(0));
}

#[test]
fn builtin_editor_refuses_a_file_over_the_size_limit() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    // 15 MB against a 10 MB limit — the two numbers differ, so swapping
    // `megabytes` with `limit_megabytes` would change the message.
    sparse_file(
        temp_dir.path(),
        "huge.txt",
        MAX_FILE_SIZE.bytes() + 5 * 1024 * 1024,
    )?;

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "huge.txt"));

    handle_edit_builtin(&mut app)?;

    assert_eq!(
        app.status_text(),
        "File too large to edit (15.0 MB, max 10 MB)"
    );
    assert!(app.get_editor().is_none(), "the file must not be loaded");
    Ok(())
}

#[test]
fn builtin_editor_opens_a_file_exactly_at_the_size_limit() -> io::Result<()> {
    // The guard is `>`, not `>=`: a file of exactly the limit still opens.
    let temp_dir = TempDir::new().expect("temp dir");
    sparse_file(temp_dir.path(), "limit.txt", MAX_FILE_SIZE.bytes())?;

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "limit.txt"));

    handle_edit_builtin(&mut app)?;

    assert_eq!(
        app.get_editor().expect("editor overlay should open").title,
        "limit.txt"
    );
    Ok(())
}

#[test]
fn builtin_editor_ignores_an_entry_without_a_path() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let mut app = build_app(&temp_dir);
    select_parent_entry(&mut app);

    handle_edit_builtin(&mut app)?;

    assert!(app.get_editor().is_none());
    assert!(app.status_text().is_empty());
    Ok(())
}

/// A file deleted after the panel was listed has no metadata, so the size
/// guard is skipped and the read failure is what reaches the status line —
/// naming the file that could not be opened.
#[test]
fn builtin_editor_reports_a_file_that_vanished_after_the_listing() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let mut app = build_app(&temp_dir);
    app.left_panel.entries = vec![DirEntry {
        name: "ghost.txt".to_string(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(0),
    }];
    app.left_panel.select(Some(0));

    handle_edit_builtin(&mut app)?;

    assert!(app.get_editor().is_none());
    let expected = format!(
        "Cannot edit {}",
        temp_dir.path().join("ghost.txt").display()
    );
    assert!(
        app.status_text().starts_with(&expected),
        "{}",
        app.status_text()
    );
    Ok(())
}

#[test]
fn test_mode_editor_ignores_an_entry_without_a_path() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let mut app = build_app(&temp_dir);
    select_parent_entry(&mut app);

    handle_edit_test_mode(&mut app, TestEditorBehavior::Success)?;

    assert!(app.status_text().is_empty());
    Ok(())
}

/// Fails on the first step of suspending the terminal, so the editor is never
/// reached.
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
    assert!(app.status_text().is_empty());
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
        launched = Some((editor.to_string(), args.to_vec()));
        assert_eq!(path, file_path.as_path());
        Ok(std::process::ExitStatus::default())
    })?;

    let (editor, args) = launched.expect("the editor runner should have been called");
    assert_eq!(editor, "ffm-fake-code");
    assert_eq!(args, ["-w", "-n"]);
    Ok(())
}
