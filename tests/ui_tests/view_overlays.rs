//! Purpose: Help and viewer overlays.

use super::*;

// ---------------------------------------------------------------------------
// Help overlay tests
// ---------------------------------------------------------------------------

#[test]
fn test_help_overlay_content() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.set_overlay(Overlay::Help);

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    assert!(
        all_text.contains("Help"),
        "Help overlay should show 'Help' title"
    );
    assert!(
        all_text.contains("F3: View"),
        "Help overlay should mention F3"
    );
    assert!(
        all_text.contains("F5: Copy"),
        "Help overlay should mention F5"
    );
}

// ---------------------------------------------------------------------------
// Viewer overlay tests
// ---------------------------------------------------------------------------

#[test]
fn test_viewer_overlay_shows_filename() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "example.txt",
        vec!["Hello world".to_owned(), "Second line".to_owned()],
    )));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    assert!(
        all_text.contains("example.txt"),
        "Viewer should show filename in title"
    );
    assert!(
        all_text.contains("Hello world"),
        "Viewer should show file content"
    );
}

#[test]
fn test_viewer_overlay_with_scroll() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    let many_lines: Vec<String> = (0..100).map(|i| format!("Line {i}")).collect();
    app.set_overlay(Overlay::Viewer(Viewer {
        scroll: ScrollTop::TOP.down(50, usize::MAX),
        ..Viewer::report("long.txt", many_lines)
    }));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    // After scrolling to 50, "Line 0" should NOT be visible
    assert!(
        !all_text.contains("Line 0 "),
        "Scrolled viewer should not show Line 0"
    );
    // But "Line 50" should be visible
    assert!(
        all_text.contains("Line 50"),
        "Scrolled viewer should show Line 50"
    );
}
