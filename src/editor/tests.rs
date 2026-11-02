//! Unit tests and shared test helpers for `editor`.

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

#[cfg(test)]
mod tests_builtin;
#[cfg(test)]
mod tests_external_impl;
#[cfg(test)]
mod tests_resume_errors;
#[cfg(test)]
mod tests_spawned_editor;
#[cfg(test)]
mod tests_test_mode;

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

/// A panel folder whose reload fails with nowhere to go. A deleted absolute
/// folder no longer fails: the panel moves to the nearest folder above it. A
/// relative path that does not exist has no folder above it at all, and it
/// fails without the process-wide `FORCE_REFRESH_ERROR`, which would also
/// fail the reloads of every other test running beside this one.
fn unreachable_folder() -> PathBuf {
    PathBuf::from("no-such-folder-for-an-editor-test")
}

#[test]
fn test_split_editor_command_splits_arguments() {
    assert_eq!(
        super::split_editor_command("code -w -n"),
        ("code".to_owned(), vec!["-w".to_owned(), "-n".to_owned()])
    );
    assert_eq!(
        super::split_editor_command(""),
        ("nano".to_owned(), Vec::new())
    );
}

#[test]
fn test_select_entry_returns_false() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("create panel");

    assert!(!select_entry(&mut panel, "missing.txt"));
    Ok(())
}

#[test]
#[expect(
    clippy::significant_drop_tightening,
    reason = "the lock is moved into set_with_lock, which holds it until the guard drops"
)]
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
