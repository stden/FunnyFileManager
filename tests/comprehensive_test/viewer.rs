//! Purpose: The built-in viewer: defaults, scrolling, closing and what it opens.

use super::*;

// ==================== Viewer ====================

#[test]
fn test_viewer_default() {
    let v = Viewer::default();
    assert!(v.title().is_empty());
    assert!(v.lines.is_empty());
    assert_eq!(v.scroll, ScrollTop::TOP);
}

// ==================== Overlay: Viewer Scroll ====================

#[test]
fn test_viewer_scroll_down_and_up() -> io::Result<()> {
    use std::fmt::Write;

    let temp = TempDir::new().unwrap();
    let mut content = String::new();
    for i in 0..50 {
        writeln!(content, "Line {i}").map_err(io::Error::other)?;
    }
    fs::write(temp.path().join("long.txt"), content.as_bytes()).unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    // Select the file
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "long.txt")
        .unwrap();
    app.left_panel.select(Some(idx));

    let events = vec![
        key(KeyCode::F(3)),  // open viewer
        key(KeyCode::Down),  // scroll down
        key(KeyCode::Down),  // scroll down again
        key(KeyCode::Up),    // scroll up
        key(KeyCode::Enter), // close viewer
        key(KeyCode::F(10)), // quit
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_viewer_scroll_up_at_zero() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("file.txt"), b"line1\nline2").unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "file.txt")
        .unwrap();
    app.left_panel.select(Some(idx));

    let events = vec![
        key(KeyCode::F(3)), // open viewer
        key(KeyCode::Up),   // scroll up at 0 - no-op
        key(KeyCode::Esc),  // close
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_viewer_close_with_f3() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("file.txt"), b"content").unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "file.txt")
        .unwrap();
    app.left_panel.select(Some(idx));

    let events = vec![
        key(KeyCode::F(3)), // open viewer
        key(KeyCode::F(3)), // close with F3
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

// ==================== handle_view edge cases ====================

#[test]
fn test_view_empty_file() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("empty.txt"), b"").unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "empty.txt")
        .unwrap();
    app.left_panel.select(Some(idx));

    let events = vec![
        key(KeyCode::F(3)), // view empty file
        key(KeyCode::Esc),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_view_directory_opens_the_information_panel() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let sub = temp.path().join("subdir");
    fs::create_dir(&sub).unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "subdir")
        .unwrap();
    app.left_panel.select(Some(idx));

    let events = vec![
        // F3 on a directory measures its total size in-place in the panel,
        // so F10 immediately reaches the panels without needing Esc first.
        key(KeyCode::F(3)),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}
