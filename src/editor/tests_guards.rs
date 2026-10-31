//! Unit tests: the guards F4 runs before it opens a file. The external
//! launcher's guards and `$EDITOR` plumbing are in `tests_launch_guards.rs`.

use super::tests::{build_app, select_entry};
use super::{handle_edit_builtin, handle_edit_test_mode, TestEditorBehavior};
use crate::models::EntryKind;
use crate::models::FileSize;
use crate::models::Modified;

use crate::models::DirEntry;
use crate::utils::MAX_FILE_SIZE;

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
    *app.left_panel.entries = vec![DirEntry {
        name: "..".to_owned(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(0),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
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
    // Real text rather than a sparse file: a sparse file reads as NUL bytes,
    // which the editor refuses as binary before the size would matter.
    let temp_dir = TempDir::new().expect("temp dir");
    let limit = usize::try_from(MAX_FILE_SIZE.bytes()).expect("the limit fits in memory");
    std::fs::write(temp_dir.path().join("limit.txt"), vec![b'a'; limit])?;

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
    assert_eq!(app.status_text().len(), 0);
    Ok(())
}

/// A file deleted after the panel was listed has no metadata, so the size
/// guard is skipped and the read failure is what reaches the status line —
/// naming the file that could not be opened.
#[test]
fn builtin_editor_reports_a_file_that_vanished_after_the_listing() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let mut app = build_app(&temp_dir);
    *app.left_panel.entries = vec![DirEntry {
        name: "ghost.txt".to_owned(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(0),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
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

    assert_eq!(app.status_text().len(), 0);
    Ok(())
}
