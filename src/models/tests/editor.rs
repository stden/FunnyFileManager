//! Purpose: Built-in editor state — content round trips, line endings, editing, saving, and the App accessors for it.

use super::*;

#[test]
fn test_editor_from_content_round_trip() {
    // CRLF files must survive untouched: F4 + F2 on a .bat or a CRLF-committed
    // source file used to rewrite every line ending and produce a whole-file diff.
    for content in [
        "",
        "a",
        "a\n",
        "\n",
        "a\nb",
        "a\nb\n",
        "a\r\nb\r\n",
        "a\r\n",
    ] {
        let editor = editor_from(content);
        assert_eq!(editor.content(), content, "round trip of {content:?}");
        assert!(!editor.state.is_modified());
    }
}

#[test]
fn test_editor_keeps_crlf_while_editing() {
    let mut editor = editor_from("alpha\r\nbeta\r\n");
    assert_eq!(editor.format.line_ending, LineEnding::Crlf);
    editor.move_end();
    editor.insert_newline();
    assert_eq!(editor.content(), "alpha\r\n\r\nbeta\r\n");
}

#[test]
fn test_editor_is_crlf_only_when_every_line_is() {
    // A single stray CRLF must not convert an otherwise LF file, and a single
    // LF must not convert a CRLF-majority one either: a mixed file is split
    // on LF, and each CRLF line keeps its CR as text.
    assert_eq!(
        editor_from("a\r\nb\nc\nd\n").format.line_ending,
        LineEnding::Lf
    );
    assert_eq!(
        editor_from("a\r\nb\r\nc\n").format.line_ending,
        LineEnding::Lf
    );
    assert_eq!(
        editor_from("a\r\nb\r\n").format.line_ending,
        LineEnding::Crlf
    );
}

#[test]
fn test_editor_empty_file_has_one_line() {
    let editor = editor_from("");
    assert_eq!(editor.lines, vec![String::new()]);
}

#[test]
fn test_editor_insert_char() {
    let mut editor = editor_from("ac\n");
    editor.move_right();
    editor.insert_char('b');
    assert_eq!(editor.content(), "abc\n");
    assert_eq!(editor.cursor_col, 2);
    assert!(editor.state.is_modified());
}

#[test]
fn test_editor_insert_newline_splits_line() {
    let mut editor = editor_from("abcd\n");
    editor.move_right();
    editor.move_right();
    editor.insert_newline();
    assert_eq!(editor.content(), "ab\ncd\n");
    assert_eq!(editor.cursor_line, 1);
    assert_eq!(editor.cursor_col, 0);
}

#[test]
fn test_editor_backspace_char_and_line_join() {
    let mut editor = editor_from("ab\ncd\n");
    editor.move_down(1);
    editor.move_right();
    editor.backspace();
    assert_eq!(editor.content(), "ab\nd\n");

    editor.backspace(); // at col 0 — join with previous line
    assert_eq!(editor.content(), "abd\n");
    assert_eq!(editor.cursor_line, 0);
    assert_eq!(editor.cursor_col, 2);

    let mut at_start = editor_from("x\n");
    at_start.backspace(); // nothing before the first char
    assert_eq!(at_start.content(), "x\n");
    assert!(!at_start.state.is_modified());
}

#[test]
fn test_editor_delete_forward_char_and_line_join() {
    let mut editor = editor_from("ab\ncd\n");
    editor.delete_forward();
    assert_eq!(editor.content(), "b\ncd\n");

    editor.move_right(); // to end of "b"
    editor.delete_forward(); // join with next line
    assert_eq!(editor.content(), "bcd\n");

    editor.move_down(1);
    editor.cursor_col = editor.lines[0].chars().count();
    editor.delete_forward(); // at end of last line — no-op
    assert_eq!(editor.content(), "bcd\n");
}

#[test]
fn test_editor_unicode_editing() {
    let mut editor = editor_from("файл\n");
    editor.move_right();
    editor.move_right();
    editor.insert_char('ё');
    assert_eq!(editor.content(), "фаёйл\n");
    editor.backspace();
    assert_eq!(editor.content(), "файл\n");
    editor.delete_forward();
    assert_eq!(editor.content(), "фал\n");
}

#[test]
fn test_editor_cursor_movement_and_clamping() {
    let mut editor = editor_from("long line\nab\n");
    editor.cursor_col = 9;
    editor.move_down(1);
    assert_eq!(editor.cursor_col, 2, "column clamps to shorter line");
    editor.move_down(100);
    assert_eq!(editor.cursor_line, 1, "clamped to last line");
    editor.move_up(100);
    assert_eq!(editor.cursor_line, 0);

    // Left from col 0 wraps to end of previous line; right from EOL wraps down.
    editor.move_down(1);
    editor.cursor_col = 0;
    editor.move_left();
    assert_eq!((editor.cursor_line, editor.cursor_col), (0, 9));
    editor.move_right();
    assert_eq!((editor.cursor_line, editor.cursor_col), (1, 0));
}

#[test]
fn test_editor_save_writes_file_and_clears_modified() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let path = temp_dir.path().join("save.txt");
    fs::write(&path, "hello\n").expect("test setup");

    let mut editor = Editor::from_content("save.txt".to_string(), path.clone(), "hello\n");
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

    let mut editor = Editor::from_content("atomic.txt".to_string(), path.clone(), "hello\n");
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
    assert_eq!(names, vec!["atomic.txt".to_string()]);
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

    let mut editor = Editor::from_content("mode.txt".to_string(), path.clone(), "hello\n");
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
