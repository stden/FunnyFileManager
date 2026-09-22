//! Unit tests and shared test helpers for `editor`.
#![allow(unused_imports)]

use super::{
    handle_edit_builtin, handle_edit_impl, handle_edit_test_mode, handle_edit_with,
    TestEditorBehavior,
};
use crate::models::EntryKind;
use crate::models::FileSize;
use crate::models::Modified;

use crate::models::{App, DirEntry, Panel, PanelSide};

use crate::terminal::TerminalOps;

use crate::test_support::{editor_env_lock, restore_editor_env, EditorEnvGuard};

use crate::WritableTestBackend;

use ratatui::Terminal;

use std::ffi::OsString;

use std::fs;

use std::io;

use std::path::{Path, PathBuf};

use tempfile::TempDir;

pub fn build_app(temp_dir: &TempDir) -> App {
    let mut app = App::with_panels(
        Panel::new(temp_dir.path().to_path_buf()).expect("create left panel"),
        Panel::new(temp_dir.path().to_path_buf()).expect("create right panel"),
    );
    // A blank status line, so a test can tell "nothing happened" apart from
    // the key hints `App::new` starts with.
    app.status = crate::status::Status::Cleared;
    app
}

pub fn select_entry(panel: &mut Panel, name: &str) -> bool {
    if let Some(index) = panel.entries.iter().position(|entry| entry.name == name) {
        panel.select(Some(index));
        true
    } else {
        false
    }
}

pub fn run_editor_success(_editor: &str, _path: &Path) -> io::Result<std::process::ExitStatus> {
    Ok(std::process::ExitStatus::default())
}

struct FailingResumeOps;

impl TerminalOps<WritableTestBackend> for FailingResumeOps {
    fn enable_raw(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn disable_raw(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn enter_alternate(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Err(io::Error::other("enter_alternate"))
    }

    fn leave_alternate(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }

    fn enable_mouse(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }

    fn disable_mouse(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }

    fn clear(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }

    fn show_cursor(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(unix)]
pub fn fake_editor_path(dir: &Path, exit_code: i32) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("fake_editor.sh");
    let script = format!("#!/bin/sh\nexit {exit_code}\n");
    fs::write(&path, script).expect("write fake editor script");
    let mut perms = fs::metadata(&path)
        .expect("read fake editor metadata")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).expect("set fake editor permissions");
    path
}

#[cfg(windows)]
pub fn fake_editor_path(dir: &Path, exit_code: i32) -> PathBuf {
    let path = dir.join("fake_editor.cmd");
    let script = format!("@echo off\r\nexit /b {exit_code}\r\n");
    fs::write(&path, script).expect("write fake editor script");
    path
}

#[cfg(unix)]
pub fn failure_exit_status() -> std::process::ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(1 << 8)
}

#[cfg(windows)]
pub fn failure_exit_status() -> std::process::ExitStatus {
    use std::os::windows::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(1)
}

#[test]
fn test_handle_edit_impl_no_selection() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let mut app = build_app(&temp_dir);
    app.left_panel.select(None);

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    let result = handle_edit_impl(&mut terminal, &mut app, |_, _, _| {
        run_editor_success("", Path::new(""))
    });

    assert!(result.is_ok());
    assert!(app.status_text().is_empty());
    Ok(())
}

#[test]
fn test_handle_edit_impl_directory_selected() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir).expect("create subdir");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "subdir"));

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    let result = handle_edit_impl(&mut terminal, &mut app, |_, _, _| {
        run_editor_success("", Path::new(""))
    });

    assert!(result.is_ok());
    assert_eq!(app.status_text(), "Editing directories is not supported");
    Ok(())
}

#[test]
fn test_handle_edit_impl_selected_path_none() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let mut app = build_app(&temp_dir);
    app.left_panel.entries = vec![DirEntry {
        name: "..".to_string(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(0),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    }];
    app.left_panel.select(Some(0));

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    let result = handle_edit_impl(&mut terminal, &mut app, |_, _, _| {
        panic!("editor should not be launched when selected_path is None")
    });

    assert!(result.is_ok());
    assert!(app.status_text().is_empty());
    Ok(())
}

#[test]
fn test_handle_edit_impl_success() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));
    let editor_path = Path::new("nano");
    let _guard = EditorEnvGuard::set(editor_path);

    let expected_editor = "nano";

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    let result = handle_edit_impl(&mut terminal, &mut app, |editor, _args, path| {
        assert_eq!(editor, expected_editor);

        assert_eq!(path, file_path.as_path());
        run_editor_success(editor, path)
    });

    assert!(result.is_ok());
    assert!(app.status_text().contains("Edited"));
    Ok(())
}

#[test]
fn test_handle_edit_impl_exit_error() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    let result = handle_edit_impl(&mut terminal, &mut app, |_, _, _| Ok(failure_exit_status()));

    assert!(result.is_ok());
    assert!(app.status_text().contains("Editor exited with status"));
    Ok(())
}

#[test]
fn test_handle_edit_impl_launch_error() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    let result = handle_edit_impl(&mut terminal, &mut app, |_, _, _| {
        Err(io::Error::new(io::ErrorKind::NotFound, "missing editor"))
    });

    assert!(result.is_ok());
    assert!(app.status_text().contains("Failed to launch editor"));
    Ok(())
}

#[test]
fn test_handle_edit_impl_propagates_reload_error() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let _guard = EditorEnvGuard::clear();

    let mut app = build_app(&temp_dir);
    app.active_panel = PanelSide::Right;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("create right panel");
    assert!(select_entry(&mut app.right_panel, "file.txt"));
    app.left_panel.path = temp_dir.path().join("missing-left-panel");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    let err = handle_edit_impl(&mut terminal, &mut app, |editor, _args, path| {
        assert_eq!(editor, "nano");
        assert_eq!(path, file_path.as_path());
        run_editor_success(editor, path)
    })
    .expect_err("missing left panel path should fail reload");

    assert_eq!(err.kind(), io::ErrorKind::NotFound);
    assert!(app.status_text().contains("Edited"));
    Ok(())
}

/// Launch the resolved editor for real — the production runner, minus the
/// terminal juggling. Tests that use it must point `$EDITOR` at a script that
/// exits immediately.
fn run_editor_process(
    editor: &str,
    args: &[String],
    path: &Path,
) -> io::Result<std::process::ExitStatus> {
    std::process::Command::new(editor)
        .args(args)
        .arg(path)
        .status()
}

/// `$EDITOR` really is spawned, and exit code 0 reports success.
#[test]
fn test_handle_edit_impl_spawns_editor_from_env() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let editor_path = fake_editor_path(temp_dir.path(), 0);
    let _guard = EditorEnvGuard::set(&editor_path);

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    handle_edit_impl(&mut terminal, &mut app, run_editor_process)?;

    assert!(
        app.status_text().contains("Edited"),
        "{}",
        app.status_text()
    );
    Ok(())
}

#[test]
fn test_handle_edit_with_surfaces_resume_error() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut terminal_ops = FailingResumeOps;

    let error = handle_edit_with(&mut terminal, &mut app, &mut terminal_ops, |_, _, _| {
        Ok(std::process::ExitStatus::default())
    })
    .expect_err("resume failure should be returned");

    assert_eq!(error.to_string(), "enter_alternate");
    assert!(app.status_text().contains("Edited"));
    Ok(())
}

#[test]
fn test_handle_edit_with_refresh_error_wins_over_resume_error() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let mut app = build_app(&temp_dir);
    app.active_panel = PanelSide::Right;
    assert!(select_entry(&mut app.right_panel, "file.txt"));
    app.left_panel.path = temp_dir.path().join("missing-left-panel");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut terminal_ops = FailingResumeOps;

    let error = handle_edit_with(&mut terminal, &mut app, &mut terminal_ops, |_, _, _| {
        Ok(std::process::ExitStatus::default())
    })
    .expect_err("refresh failure should be returned");

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert!(app.status_text().contains("Edited"));
    Ok(())
}

/// A non-zero exit from the spawned editor is reported, not swallowed.
#[test]
fn test_handle_edit_impl_spawned_editor_exit_error() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let editor_path = fake_editor_path(temp_dir.path(), 1);
    let _guard = EditorEnvGuard::set(&editor_path);

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    handle_edit_impl(&mut terminal, &mut app, run_editor_process)?;

    assert!(
        app.status_text().contains("Editor exited with status"),
        "{}",
        app.status_text()
    );
    Ok(())
}

/// An editor path containing a space must still launch: the `$EDITOR`
/// whitespace split may not chop a real program path in half.
#[test]
fn test_handle_edit_impl_editor_path_with_space() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let editor_dir = temp_dir.path().join("Program Files");
    fs::create_dir(&editor_dir).expect("create editor dir");
    let editor_path = fake_editor_path(&editor_dir, 0);
    let _guard = EditorEnvGuard::set(&editor_path);

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    handle_edit_impl(&mut terminal, &mut app, run_editor_process)?;

    assert!(
        app.status_text().contains("Edited"),
        "{}",
        app.status_text()
    );
    Ok(())
}

#[test]
fn test_split_editor_command_splits_arguments() {
    assert_eq!(
        super::split_editor_command("code -w -n"),
        ("code".to_string(), vec!["-w".to_string(), "-n".to_string()])
    );
    assert_eq!(
        super::split_editor_command(""),
        ("nano".to_string(), Vec::new())
    );
}

#[test]
fn test_handle_edit_test_mode_is_safe() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));

    let result = handle_edit_test_mode(&mut app, TestEditorBehavior::Success);

    assert!(result.is_ok());
    assert!(app.status_text().contains("Edited"));
    Ok(())
}

/// No selection: `handle_edit_test_mode` returns early without touching the
/// status line, mirroring `handle_edit_impl`'s own no-selection guard.
#[test]
fn test_handle_edit_test_mode_no_selection() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let mut app = build_app(&temp_dir);
    app.left_panel.select(None);

    let result = handle_edit_test_mode(&mut app, TestEditorBehavior::Success);

    assert!(result.is_ok());
    assert!(app.status_text().is_empty());
    Ok(())
}

/// A directory selection is rejected before any behavior is simulated.
#[test]
fn test_handle_edit_test_mode_directory_selected() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir).expect("create subdir");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "subdir"));

    let result = handle_edit_test_mode(&mut app, TestEditorBehavior::Success);

    assert!(result.is_ok());
    assert_eq!(app.status_text(), "Editing directories is not supported");
    Ok(())
}

/// `TestEditorBehavior::FailLaunch` reports the same status text a real
/// launch failure would produce.
#[test]
fn test_handle_edit_test_mode_fail_launch() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));

    let result = handle_edit_test_mode(&mut app, TestEditorBehavior::FailLaunch);

    assert!(result.is_ok());
    assert_eq!(
        app.status_text(),
        "Failed to launch editor: No such file or directory (os error 2)"
    );
    Ok(())
}

/// `TestEditorBehavior::ExitError` reports the same status text a
/// non-zero editor exit would produce.
#[test]
fn test_handle_edit_test_mode_exit_error() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));

    let result = handle_edit_test_mode(&mut app, TestEditorBehavior::ExitError);

    assert!(result.is_ok());
    assert_eq!(
        app.status_text(),
        "Editor exited with status exit status: 1"
    );
    Ok(())
}

#[test]
fn test_select_entry_returns_false() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("create panel");

    assert!(!select_entry(&mut panel, "missing.txt"));
    Ok(())
}

#[test]
#[allow(clippy::significant_drop_tightening)] // lock is moved into set_with_lock
fn test_editor_env_guard_restores_existing_value() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let editor_path = fake_editor_path(temp_dir.path(), 0);

    let lock = editor_env_lock();
    let original = std::env::var_os("EDITOR");
    std::env::set_var("EDITOR", "keep");

    let guard = EditorEnvGuard::set_with_lock(lock, &editor_path);
    let current = std::env::var("EDITOR").unwrap();
    let expected = editor_path.to_string_lossy().to_string();
    assert_eq!(current, expected);
    drop(guard);

    let lock = editor_env_lock();
    assert_eq!(std::env::var("EDITOR").unwrap(), "keep");
    restore_editor_env(Some(OsString::from("ffm_temp")));
    restore_editor_env(None);
    restore_editor_env(original);
    drop(lock);

    Ok(())
}

#[test]
fn test_handle_edit_impl_default_editor() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));

    // Remove EDITOR env var to test default
    let _guard = EditorEnvGuard::clear();

    let expected_editor = "nano";

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    let result = handle_edit_impl(&mut terminal, &mut app, |editor, _args, path| {
        assert_eq!(editor, expected_editor);

        assert_eq!(path, file_path.as_path());
        run_editor_success(editor, path)
    });

    assert!(result.is_ok());
    assert!(app.status_text().contains("Edited"));

    Ok(())
}

#[test]
fn test_handle_edit_builtin_opens_editor_overlay() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, "line one\nline two\n").expect("write file");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));

    handle_edit_builtin(&mut app)?;

    let editor = app.get_editor().expect("editor overlay should be open");
    assert_eq!(editor.title, "file.txt");
    assert_eq!(editor.path, file_path);
    assert_eq!(editor.lines, vec!["line one", "line two"]);
    assert!(!editor.state.is_modified());
    assert!(app.status_text().contains("Editing"));
    Ok(())
}

#[test]
fn test_handle_edit_builtin_no_selection() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let mut app = build_app(&temp_dir);
    app.left_panel.select(None);

    handle_edit_builtin(&mut app)?;

    assert!(app.get_editor().is_none());
    assert!(app.status_text().is_empty());
    Ok(())
}

#[test]
fn test_handle_edit_builtin_directory_selected() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    fs::create_dir(temp_dir.path().join("subdir")).expect("create subdir");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "subdir"));

    handle_edit_builtin(&mut app)?;

    assert!(app.get_editor().is_none());
    assert_eq!(app.status_text(), "Editing directories is not supported");
    Ok(())
}

#[test]
fn test_handle_edit_builtin_rejects_malformed_utf16() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    fs::write(temp_dir.path().join("bin.dat"), [0xFF, 0xFE, 0x00, 0xD8]).expect("write file");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "bin.dat"));

    handle_edit_builtin(&mut app)?;

    assert!(app.get_editor().is_none());
    assert!(app.status_text().contains("Cannot edit"));
    Ok(())
}

#[test]
fn editing_records_the_file_in_the_view_history() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("edited.txt");
    fs::write(&file_path, "text\n").expect("write file");
    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "edited.txt"));
    handle_edit_builtin(&mut app)?;
    assert_eq!(app.view_history.last(), Some(file_path.as_path()));

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "edited.txt"));
    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;
    handle_edit_impl(&mut terminal, &mut app, |_, _, _| {
        Err(io::Error::other("no editor"))
    })?;
    assert_eq!(
        app.view_history.last(),
        Some(file_path.as_path()),
        "remembered on launch even when the editor fails"
    );
    Ok(())
}
