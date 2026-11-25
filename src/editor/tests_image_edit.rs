//! Purpose: Unit tests for F4 launching the registered image editor on image files.

use std::fs;
use std::io;

use tempfile::TempDir;

use super::handle_edit_builtin;
use super::tests::{build_app, select_entry};
use crate::models::{DirEntry, EntryKind, Panel};
use crate::ops::system_open::{fail_next_edit, take_edited};
use crate::status::Status;

#[test]
fn f4_on_an_image_launches_the_image_editor_and_records_view_history() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let jpg = temp_dir.path().join("denis.jpg");
    fs::write(&jpg, b"\xff\xd8\xff\xd9")?;
    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "denis.jpg"));

    drop(take_edited());
    handle_edit_builtin(&mut app)?;

    assert!(app.get_editor().is_none());
    assert_eq!(take_edited(), Some(std::path::absolute(&jpg)?));
    assert_eq!(app.status, Status::Editing(jpg.display().to_string()));
    assert_eq!(app.view_history.last(), Some(jpg.as_path()));
    Ok(())
}

#[test]
fn f4_on_svg_still_opens_builtin_text_editor() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("icon.svg"), b"<svg/>")?;
    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "icon.svg"));

    drop(take_edited());
    handle_edit_builtin(&mut app)?;

    assert!(take_edited().is_none());
    assert_eq!(
        app.get_editor().expect("SVG opens as text").lines,
        ["<svg/>"]
    );
    Ok(())
}

#[test]
fn f4_on_an_image_reports_failures_and_non_regular_targets() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let png = temp_dir.path().join("x.png");
    fs::write(&png, b"png")?;
    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "x.png"));

    drop(take_edited());
    fail_next_edit();
    handle_edit_builtin(&mut app)?;
    assert!(take_edited().is_none());
    assert!(matches!(app.status, Status::CannotEdit { ref path, .. } if path == &png));

    let dir_img = temp_dir.path().join("dir.png");
    fs::create_dir(&dir_img)?;
    app.left_panel = Panel::from_entries(
        temp_dir.path().to_owned(),
        vec![DirEntry {
            name: "dir.png".to_owned(),
            kind: EntryKind::File,
            ..DirEntry::default()
        }],
    );
    app.left_panel.select(Some(0));
    handle_edit_builtin(&mut app)?;
    assert_eq!(
        app.status,
        Status::NotRegularFile(dir_img.display().to_string())
    );
    Ok(())
}
