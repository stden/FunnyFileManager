//! Purpose: unit tests for opening a file at a line, as Find duplicate blocks opens a copy.

use std::fs;
use std::io;

use tempfile::TempDir;

use super::builtin::open_at_line;
use super::tests::build_app;
use crate::models::{App, Overlay};
use crate::status::Status;

/// A file of ten numbered lines, in `temp_dir`.
fn ten_lines(temp_dir: &TempDir) -> io::Result<std::path::PathBuf> {
    let path = temp_dir.path().join("ten.rs");
    let text = (1..=10)
        .map(|n| format!("line {n}"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&path, text)?;
    Ok(path)
}

fn cursor_line(app: &App) -> Option<usize> {
    match &app.overlay {
        Overlay::Editor(editor) => Some(editor.cursor_line),
        _ => None,
    }
}

#[test]
fn the_cursor_lands_on_the_line_asked_for() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let path = ten_lines(&temp_dir)?;
    let mut app = build_app(&temp_dir);
    open_at_line(&mut app, &path, 5);
    assert_eq!(cursor_line(&app), Some(4), "line 5 is index 4");
    Ok(())
}

#[test]
fn a_line_past_the_end_lands_on_the_last_line() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let path = ten_lines(&temp_dir)?;
    let mut app = build_app(&temp_dir);
    open_at_line(&mut app, &path, 99);
    assert_eq!(cursor_line(&app), Some(9));
    Ok(())
}

#[test]
fn a_file_the_editor_refuses_opens_nothing_and_says_so() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let path = temp_dir.path().join("blob.bin");
    fs::write(&path, b"MZ\x00\x01")?;
    let mut app = build_app(&temp_dir);
    open_at_line(&mut app, &path, 1);
    assert!(!app.overlay.is_active());
    assert!(!matches!(app.status, Status::Editing(_)));
    Ok(())
}
