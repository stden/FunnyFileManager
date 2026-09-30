//! Purpose: Popup input: viewer, help, delete confirmation, new-file input and command line.

use super::*;

// ═══════════════════════════════════════════════════════════════════
// process_popup_input — edge cases
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_popup_viewer_scroll_down() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "Test",
        (0..30).map(|i| format!("line {i}")).collect(),
    )));
    process_popup_input(&mut app, key_event(KeyCode::Down));
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 1);
    Ok(())
}

#[test]
fn test_popup_viewer_scroll_up() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::Viewer(Viewer {
        scroll: ScrollTop::TOP.down(5, usize::MAX),
        ..Viewer::report("Test", (0..30).map(|i| format!("line {i}")).collect())
    }));
    process_popup_input(&mut app, key_event(KeyCode::Up));
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 4);
    Ok(())
}

#[test]
fn test_popup_viewer_close_with_esc() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "Close",
        vec!["x".to_owned()],
    )));
    process_popup_input(&mut app, key_event(KeyCode::Esc));
    assert!(app.get_viewer().is_none());
    Ok(())
}

#[test]
fn test_popup_help_close() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::Help);
    process_popup_input(&mut app, key_event(KeyCode::Esc));
    assert!(!matches!(app.overlay, Overlay::Help));
    Ok(())
}

#[test]
fn test_popup_delete_confirm_yes() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = temp.path().join("del.txt");
    fs::write(&path, "bye")?;
    let mut app = setup_app(&temp)?;
    app.set_overlay(Overlay::DeleteConfirm(vec![path.clone()]));
    process_popup_input(&mut app, key_event(KeyCode::Char('y')));
    assert!(app.get_delete_confirm().is_none());
    assert!(!path.exists());
    Ok(())
}

#[test]
fn test_popup_delete_confirm_no() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = temp.path().join("keep.txt");
    fs::write(&path, "stay")?;
    let mut app = setup_app(&temp)?;
    app.set_overlay(Overlay::DeleteConfirm(vec![path.clone()]));
    process_popup_input(&mut app, key_event(KeyCode::Char('n')));
    assert!(app.get_delete_confirm().is_none());
    assert!(path.exists());
    Ok(())
}

#[test]
fn test_popup_new_file_input_typing() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::NewFileInput(String::new().into()));
    process_popup_input(&mut app, key_event(KeyCode::Char('a')));
    assert_eq!(app.get_new_file_input(), Some("a"));
    process_popup_input(&mut app, key_event(KeyCode::Char('b')));
    assert_eq!(app.get_new_file_input(), Some("ab"));
    process_popup_input(&mut app, key_event(KeyCode::Backspace));
    assert_eq!(app.get_new_file_input(), Some("a"));
    Ok(())
}

#[test]
fn test_popup_new_file_esc_cancels() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::NewFileInput("partial".to_owned().into()));
    process_popup_input(&mut app, key_event(KeyCode::Esc));
    assert!(app.get_new_file_input().is_none());
    Ok(())
}

#[test]
fn test_popup_new_file_enter_creates() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = setup_app(&temp)?;
    app.set_overlay(Overlay::NewFileInput("brand_new.txt".to_owned().into()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(temp.path().join("brand_new.txt").exists());
    assert!(app.get_new_file_input().is_none());
    Ok(())
}

#[test]
fn test_popup_command_line_backspace() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::CommandLine("abc".into()));
    process_popup_input(&mut app, key_event(KeyCode::Backspace));
    assert_eq!(app.get_command_line(), Some("ab"));
    Ok(())
}

#[test]
fn test_popup_command_line_esc() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::CommandLine("partial".into()));
    process_popup_input(&mut app, key_event(KeyCode::Esc));
    assert!(app.get_command_line().is_none());
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// Targeted coverage for specific uncovered lines
// ═══════════════════════════════════════════════════════════════════

// handlers.rs:862 — delete confirm on ".." entry
#[test]
fn test_delete_confirm_on_parent_entry_does_nothing() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("file.txt"), "data")?;
    let mut app = setup_app(&temp)?;
    app.left_panel.select(Some(0)); // ".."
    handle_delete_confirm(&mut app)?;
    assert!(app.get_delete_confirm().is_none());
    Ok(())
}

// handlers.rs:870 — delete confirm with no selection
#[test]
fn test_delete_confirm_no_selection() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = setup_app(&temp)?;
    app.left_panel.select(None);
    handle_delete_confirm(&mut app)?;
    assert!(app.get_delete_confirm().is_none());
    Ok(())
}
