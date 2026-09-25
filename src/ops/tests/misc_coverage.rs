//! Unit tests: misc coverage.

use crate::ops::file::{rename_or_fail, RENAME_TEST_MUTEX};

use crate::ops::shell::{run_shell_command, windows_shell_args};

use super::support::*;
use crate::ops::*;
use crate::popup::process_popup_input;

use crate::models::{App, Overlay, Panel};

use crossterm::event::KeyCode;

use std::fs::{self, File};

use std::io;

use tempfile::TempDir;

#[test]
fn test_get_source_paths_nothing() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;
    app.left_panel.entries.clear();
    app.left_panel.select(None);

    let paths = get_source_paths(&app);
    assert!(paths.is_empty());
    Ok(())
}

#[test]
fn test_view_empty_file_shows_placeholder() -> io::Result<()> {
    let temp = TempDir::new()?;
    File::create(temp.path().join("empty.txt"))?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "empty.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_view(&mut app)?;
    assert!(app.get_viewer().is_some());
    assert_eq!(app.get_viewer().unwrap().lines, vec!["<empty file>"]);
    Ok(())
}

#[test]
fn test_new_file_create_via_overlay() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;

    app.set_overlay(Overlay::NewFileInput("test_new.txt".to_owned()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(temp.path().join("test_new.txt").exists());
    assert!(app.status_text().contains("Created"));
    Ok(())
}

#[test]
fn test_new_file_empty_name_rejected() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::NewFileInput(String::new()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(app.status_text().contains("empty"));
    Ok(())
}

#[test]
fn test_new_file_already_exists() -> io::Result<()> {
    let temp = TempDir::new()?;
    File::create(temp.path().join("exists.txt"))?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;
    app.set_overlay(Overlay::NewFileInput("exists.txt".to_owned()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(app.status_text().contains("already exists"));
    Ok(())
}

#[test]
fn test_run_shell_command_basic() -> io::Result<()> {
    let temp = TempDir::new()?;
    let output = run_shell_command("echo test123", temp.path()).map_err(io::Error::other)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test123"));
    Ok(())
}

#[test]
fn test_windows_shell_args_include_autorun_bypass() {
    assert_eq!(windows_shell_args("echo hello"), ["/D", "/C", "echo hello"]);
}

#[test]
fn test_rename_or_fail_success() -> io::Result<()> {
    let _lock = RENAME_TEST_MUTEX
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let temp = TempDir::new()?;
    let src = temp.path().join("src.txt");
    let dst = temp.path().join("dst.txt");
    fs::write(&src, b"data")?;

    rename_or_fail(&src, &dst)?;
    assert!(!src.exists());
    assert!(dst.exists());
    Ok(())
}

#[test]
fn test_rename_or_fail_forced_failure() -> io::Result<()> {
    let temp = TempDir::new()?;
    let src = temp.path().join("src.txt");
    let dst = temp.path().join("dst.txt");
    fs::write(&src, b"data")?;

    let _guard = ForceRenameFailureGuard::enable();
    let err = rename_or_fail(&src, &dst);
    assert!(err.is_err());
    assert!(src.exists()); // Source still there
    Ok(())
}

#[test]
fn test_jump_to_match_no_match() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;

    for i in 0..5 {
        File::create(temp_dir.path().join(format!("file{i}.txt")))?;
    }
    panel.load_entries()?;

    let found = panel.find_first_matching("nonexistent");
    assert!(!found);

    Ok(())
}
