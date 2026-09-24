//! Purpose: The new-file input overlay: typing, creating, rejecting and cancelling.

use super::*;

// ──────────────────────────────────────────────
// New file input overlay (Shift+F4)
// ──────────────────────────────────────────────

#[test]
fn test_new_file_input_type_and_create() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Open new file input
    app.set_overlay(Overlay::NewFileInput(String::new()));

    // Type "hello.txt"
    for ch in "hello.txt".chars() {
        process_popup_input(&mut app, key(KeyCode::Char(ch)));
    }
    assert_eq!(app.get_new_file_input(), Some("hello.txt"));

    // Press Enter to create
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.get_new_file_input().is_none());
    assert!(temp.path().join("hello.txt").exists());
    assert!(app.status_text().contains("Created file"));

    Ok(())
}

#[test]
fn test_new_file_input_empty_name_rejected() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::NewFileInput(String::new()));

    // Press Enter with empty name
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.get_new_file_input().is_none());
    assert!(app.status_text().contains("empty"));

    Ok(())
}

#[test]
fn test_new_file_input_already_exists() -> io::Result<()> {
    let temp = TempDir::new()?;
    File::create(temp.path().join("existing.txt"))?.write_all(b"x")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;
    app.set_overlay(Overlay::NewFileInput("existing.txt".to_owned()));

    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("already exists"));

    Ok(())
}

#[test]
fn test_new_file_input_backspace() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::NewFileInput("abc".to_owned()));

    process_popup_input(&mut app, key(KeyCode::Backspace));
    assert_eq!(app.get_new_file_input(), Some("ab"));

    Ok(())
}

#[test]
fn test_new_file_input_escape_cancels() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::NewFileInput("something.txt".to_owned()));

    process_popup_input(&mut app, key(KeyCode::Esc));
    assert!(app.get_new_file_input().is_none());

    Ok(())
}

#[test]
fn test_new_file_input_ignores_ctrl_chars() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::NewFileInput("test".to_owned()));

    // Ctrl+A should not add 'a' to input
    process_popup_input(&mut app, key_ctrl(KeyCode::Char('a')));
    assert_eq!(app.get_new_file_input(), Some("test"));

    Ok(())
}
