//! Unit tests: what F4 agrees to open in the built-in editor, and what it
//! records about the file when it does.

use std::fs;
use std::io;

use tempfile::TempDir;

use super::handle_edit_builtin;
use super::tests::{build_app, select_entry};

/// F4 on `name` holding `bytes`, in a fresh app over a temporary directory.
fn open(temp_dir: &TempDir, name: &str, bytes: &[u8]) -> io::Result<crate::models::App> {
    fs::write(temp_dir.path().join(name), bytes)?;
    let mut app = build_app(temp_dir);
    assert!(select_entry(&mut app.left_panel, name), "test setup");
    handle_edit_builtin(&mut app)?;
    Ok(app)
}

/// A NUL byte is the binary heuristic F3 and Quick view already use. F4
/// opened such a file as Windows-1251 text instead, and a save wrote it back
/// through the text buffer: 65536 bytes became 65534.
#[test]
fn a_binary_file_is_refused_rather_than_opened_as_text() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let bytes = b"MZ\x90\x00\x03\r\n\x00\xff\rdata\n";

    let app = open(&temp_dir, "blob.bin", bytes)?;

    assert!(app.get_editor().is_none(), "the binary must not be loaded");
    let status = app.status_text();
    assert!(status.starts_with("Cannot edit"), "{status}");
    assert!(status.contains("binary"), "{status}");
    assert_eq!(fs::read(temp_dir.path().join("blob.bin"))?, bytes);
    Ok(())
}

/// UTF-16 text is full of NUL *bytes*, but its BOM says what it is and the
/// decoded text has none: it is not binary and still opens.
#[test]
fn utf16_text_still_opens_although_its_bytes_hold_nuls() -> io::Result<()> {
    let temp_dir = TempDir::new()?;

    let app = open(&temp_dir, "wide.txt", b"\xff\xfeH\x00i\x00\n\x00")?;

    let editor = app.get_editor().expect("UTF-16 text opens");
    assert_eq!(editor.lines, ["Hi"]);
    Ok(())
}

/// Far's Alt+F11 is the history of files viewed *and* edited. F4 recorded the
/// file only in the persisted history, which the dialog never read; the
/// dialog read a second, session-only list that only F3 fed.
#[test]
fn a_file_opened_with_f4_is_listed_by_the_view_history() -> io::Result<()> {
    use crate::models::{MenuDialog, Overlay};

    let temp_dir = TempDir::new()?;
    let mut app = open(&temp_dir, "leaf.txt", b"leaf\n")?;
    app.close_overlay();

    crate::ops::menu::execute_tool(&mut app, crate::action::ToolCommand::ViewHistory)?;

    let Overlay::MenuDialog(dialog) = &app.overlay else {
        panic!("the view history is open");
    };
    let MenuDialog::Choices { rows, .. } = dialog.as_ref() else {
        panic!("the view history lists choices");
    };
    let edited = temp_dir.path().join("leaf.txt").display().to_string();
    assert!(
        rows.iter().any(|row| row.label == edited),
        "{edited} is missing from {:?}",
        rows.iter().map(|row| &row.label).collect::<Vec<_>>()
    );
    Ok(())
}
