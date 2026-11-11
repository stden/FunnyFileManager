//! Purpose: Console mode hides the panels and paints the terminal black.

use super::*;

// ---------------------------------------------------------------------------
// Console mode tests
// ---------------------------------------------------------------------------

#[test]
fn test_console_mode_hides_panels() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.view = ViewMode::Console;

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // In console mode the screen is a console: black, with the prompt on the
    // row after the newest transcript line -- here the very first one, since
    // nothing has been run yet.
    let prompt_row = row_text(&terminal, 0);
    assert!(
        prompt_row.starts_with('●'),
        "Console mode should show a prompt, got: '{prompt_row}'"
    );
    assert_eq!(
        row_text(&terminal, 23),
        "",
        "nothing is pinned to the bottom of a console"
    );

    // Panels should NOT appear
    let mut all_text = String::new();
    for y in 0..23 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }
    assert!(
        !all_text.contains("readme.md"),
        "Console mode should hide panels"
    );
}

#[test]
fn test_console_mode_black_background() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.view = ViewMode::Console;

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // Middle of the screen should have black background
    let bg = cell_bg(&terminal, 10, 10);
    assert_eq!(bg, Color::Black, "Console mode background should be black");
}
