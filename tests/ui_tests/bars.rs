//! Purpose: Status bar, function-key bar and status messages.

use super::*;

#[test]
fn test_status_bar_shows_selected_file_info() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    // Select readme.md (index 2)
    app.left_panel.select(Some(2));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let info = row_text(&terminal, 20);
    assert!(
        info.contains("readme.md"),
        "Info row should show the selected filename, got: '{info}'"
    );
}

#[test]
fn test_status_bar_with_quick_search() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.quick_search = QuickSearch::Active("main".to_owned());

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let info = row_text(&terminal, 20);
    assert!(
        info.contains("Search: main"),
        "Info row should show the quick search text, got: '{info}'"
    );
}

// ---------------------------------------------------------------------------
// F-key bar tests
// ---------------------------------------------------------------------------

#[test]
fn test_fkey_bar_shows_all_keys() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let fkey = row_text(&terminal, 23);
    for label in &["Help", "View", "Edit", "Copy", "Delete", "Quit"] {
        assert!(
            fkey.contains(label),
            "F-key bar should contain '{label}', got: '{fkey}'"
        );
    }
}

#[test]
fn test_fkey_bar_shows_key_numbers() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let fkey = row_text(&terminal, 23);
    assert!(fkey.contains('1'), "F-key bar should show key number 1");
    assert!(fkey.contains("10"), "F-key bar should show key number 10");
}

// ---------------------------------------------------------------------------
// Status message display
// ---------------------------------------------------------------------------

#[test]
fn test_status_message_displayed() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.set_status(Status::CommandOutput(
        "Copy completed successfully".to_owned(),
    ));
    app.left_panel.select(Some(1));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let info = row_text(&terminal, 20);
    assert!(
        info.contains("Copy completed successfully"),
        "The active panel's info row should carry the message, got: '{info}'"
    );
}
