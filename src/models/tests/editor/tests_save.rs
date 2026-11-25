//! Purpose: unit tests for saving the editor's file, and the app's editor
//! helpers.

use super::*;

#[test]
fn test_editor_save_writes_file_and_clears_modified() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let path = temp_dir.path().join("save.txt");
    fs::write(&path, "hello\n").expect("test setup");

    let mut editor = Editor::from_content("save.txt".to_owned(), path.clone(), "hello\n");
    editor.cursor_col = 5;
    editor.insert_char('!');
    assert!(editor.state.is_modified());

    editor.save()?;
    assert!(!editor.state.is_modified());
    assert_eq!(fs::read_to_string(&path)?, "hello!\n");
    Ok(())
}

#[test]
fn test_editor_save_leaves_no_temp_file_behind() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let path = temp_dir.path().join("atomic.txt");
    fs::write(&path, "hello\n").expect("test setup");

    let mut editor = Editor::from_content("atomic.txt".to_owned(), path.clone(), "hello\n");
    editor.move_end();
    editor.insert_char('!');
    editor.save()?;

    assert_eq!(fs::read_to_string(&path)?, "hello!\n");
    let names: Vec<String> = fs::read_dir(temp_dir.path())?
        .map(|entry| {
            entry
                .expect("dir entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(names, vec!["atomic.txt".to_owned()]);
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_editor_save_replaces_file_and_keeps_mode() -> io::Result<()> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let temp_dir = TempDir::new().expect("test setup");
    let path = temp_dir.path().join("mode.txt");
    fs::write(&path, "hello\n").expect("test setup");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).expect("test setup");

    let mut editor = Editor::from_content("mode.txt".to_owned(), path.clone(), "hello\n");
    editor.move_end();
    editor.insert_char('!');
    let inode_before = fs::metadata(&path)?.ino();
    editor.save()?;

    let after = fs::metadata(&path)?;
    // A rename swaps the file; an in-place truncate would keep the same inode
    // and would have destroyed the original before writing a single byte.
    assert_ne!(inode_before, after.ino());
    assert_eq!(after.permissions().mode() & 0o777, 0o640);
    Ok(())
}

#[test]
fn test_app_get_editor_helpers() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    assert!(app.get_editor().is_none());
    assert!(app.get_editor_mut().is_none());

    app.set_overlay(Overlay::Editor(editor_from("x\n")));
    assert!(app.get_editor().is_some());
    app.get_editor_mut().expect("editor").insert_char('y');
    assert_eq!(app.get_editor().expect("editor").content(), "yx\n");
    Ok(())
}
