//! Purpose: Screen layout rows, centered rectangles and extreme terminal sizes.

use super::*;

// ---------------------------------------------------------------------------
// Layout structure tests
// ---------------------------------------------------------------------------

#[test]
fn test_layout_row_count() {
    // On a 24-row terminal the layout is Far's: the menu bar at the top, and
    // two rows below the panels.
    //   row 0       = menu bar, with the clock at its right end
    //   row 1       = panel top border, carrying the path
    //   row 2       = column headers (Name | Size | Date | Time)
    //   rows 3..19  = entries
    //   row 20      = panel info row (current entry, or a message)
    //   row 21      = panel bottom border, carrying the totals
    //   row 22      = command line
    //   row 23      = F-key bar
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // Row 0 is the menu bar, as it is in Far.
    let menu = row_text(&terminal, 0);
    assert!(
        menu.contains("Left"),
        "Row 0 is the menu bar, got: '{menu}'"
    );

    // Row 1 begins the panels, inside the double-line frame Far draws
    // everything with.
    let top_row = row_text(&terminal, 1);
    assert!(
        top_row.starts_with('╔'),
        "Row 1 should be the panel's top border, got: '{top_row}'"
    );
    assert!(!top_row.contains("Far Manager style"), "{top_row}");

    // Last row (23): F-key bar contains "Help" and "Quit"
    let fkey = row_text(&terminal, 23);
    assert!(
        fkey.contains("Help"),
        "F-key bar should contain 'Help', got: '{fkey}'"
    );
    assert!(
        fkey.contains("Quit"),
        "F-key bar should contain 'Quit', got: '{fkey}'"
    );
}

// ---------------------------------------------------------------------------
// Centered rect utility tests
// ---------------------------------------------------------------------------

#[test]
fn test_centered_rect_is_centered() {
    let base = Rect::new(0, 0, 100, 100);
    let rect = centered_rect(50, 50, base);

    // Should be roughly centered
    assert!(
        rect.x >= 20 && rect.x <= 30,
        "x should be ~25, got {}",
        rect.x
    );
    assert!(
        rect.y >= 20 && rect.y <= 30,
        "y should be ~25, got {}",
        rect.y
    );
    assert_eq!(rect.width, 50);
    assert_eq!(rect.height, 50);
}

#[test]
fn test_centered_rect_clamps_percent_x() {
    let base = Rect::new(0, 0, 100, 100);
    // percent_x > 100 should be clamped
    let rect = centered_rect(150, 50, base);
    assert_eq!(rect.width, 100);
}

// ---------------------------------------------------------------------------
// Minimal / edge-case terminal sizes
// ---------------------------------------------------------------------------

#[test]
fn test_tiny_terminal_does_not_panic() {
    let backend = TestBackend::new(20, 5);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    // Should not panic even on very small terminal
    terminal.draw(|f| ui(f, &mut app)).unwrap();
}

#[test]
fn test_wide_terminal() {
    let backend = TestBackend::new(200, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // Command line should fill the width with padding
    let cmd = row_text(&terminal, 22);
    assert!(
        has_prompt(&cmd),
        "Wide terminal command line works: '{cmd}'"
    );
}
