//! Purpose: Popup input: viewer scrolling and paging, and the command line.

use super::*;

const fn key_event(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

#[test]
fn test_process_popup_input_viewer_scroll() -> io::Result<()> {
    let mut app = App::new()?;

    // Create a viewer with multiple lines
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "test.txt",
        vec![
            "Line 1".to_owned(),
            "Line 2".to_owned(),
            "Line 3".to_owned(),
            "Line 4".to_owned(),
            "Line 5".to_owned(),
        ],
    )));
    app.set_viewport(funny_file_manager::models::Viewport::new(2));

    // Test scrolling down
    process_popup_input(&mut app, key_event(KeyCode::Down));
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 1);

    process_popup_input(&mut app, key_event(KeyCode::Down));
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 2);

    // Test scrolling up
    process_popup_input(&mut app, key_event(KeyCode::Up));
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 1);

    process_popup_input(&mut app, key_event(KeyCode::Up));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);

    // Test scrolling up at top (should stay at 0)
    process_popup_input(&mut app, key_event(KeyCode::Up));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);

    Ok(())
}

#[test]
fn test_process_popup_input_viewer_page_navigation() -> io::Result<()> {
    let mut app = App::new()?;

    // Create a viewer with many lines
    let lines: Vec<String> = (1..=100).map(|i| format!("Line {i}")).collect();
    app.set_overlay(Overlay::Viewer(Viewer::report("test.txt", lines)));

    // The page is the body height of the last frame, so draw one: 24 rows
    // less the status line leaves 23.
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("test terminal");
    terminal.draw(|f| ui(f, &mut app)).expect("draw");

    process_popup_input(&mut app, key_event(KeyCode::PageDown));
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 23);
    process_popup_input(&mut app, key_event(KeyCode::PageUp));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);

    // End stops with the last line on the bottom row, Home returns to the top.
    process_popup_input(&mut app, key_event(KeyCode::End));
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 77);
    process_popup_input(&mut app, key_event(KeyCode::Home));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);

    Ok(())
}

#[test]
fn test_process_popup_input_command_line() -> io::Result<()> {
    let mut app = App::new()?;

    // Start command line
    app.set_overlay(Overlay::CommandLine(String::new().into()));

    // Type characters
    process_popup_input(&mut app, key_event(KeyCode::Char('c')));
    process_popup_input(&mut app, key_event(KeyCode::Char('d')));
    process_popup_input(&mut app, key_event(KeyCode::Char(' ')));
    process_popup_input(&mut app, key_event(KeyCode::Char('/')));

    assert_eq!(app.get_command_line(), Some("cd /"));

    // Test backspace
    process_popup_input(&mut app, key_event(KeyCode::Backspace));
    assert_eq!(app.get_command_line(), Some("cd "));

    // Test escape cancels
    process_popup_input(&mut app, key_event(KeyCode::Esc));
    assert!(app.get_command_line().is_none());

    Ok(())
}
