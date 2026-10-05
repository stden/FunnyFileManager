//! Unit tests: the panel page size the chrome leaves on every terminal height,
//! and `centered_rect` at ordinary, small and degenerate sizes.

use super::*;

#[test]
fn test_panel_page_size_is_terminal_height_minus_chrome_and_borders() {
    // 24 rows - 3 chrome lines (menu bar, command line, F-key bar) - 2 panel
    // borders - 1 column-header row - 1 column rule - 1 info row = 16 entry
    // rows, which is exactly what `render_panel_contents` draws: a page must
    // move by what is on screen, or PgDn skips the first entry of every page.
    //
    // The status bar's line came back to the panels and was spent there: Far
    // puts a column header at the top and the current entry's name at the
    // bottom, and both live inside the frame.
    assert_eq!(panel_page_size(24), 16);
    assert_eq!(panel_page_size(50), 42);
}

#[test]
fn test_panel_page_size_never_returns_zero_on_tiny_terminals() {
    // A zero page size would make PgUp/PgDn silently do nothing.
    assert_eq!(panel_page_size(8), 1);
    assert_eq!(panel_page_size(6), 1);
    assert_eq!(panel_page_size(1), 1);
    assert_eq!(panel_page_size(0), 1);
}

#[test]
fn test_centered_rect_various_sizes() {
    let base = Rect {
        x: 0,
        y: 0,
        width: 100,
        height: 100,
    };

    // Test different percentages
    let rect1 = centered_rect(80, 60, base);
    assert_eq!(rect1.width, 80);
    assert_eq!(rect1.height, 60);

    let rect2 = centered_rect(100, 100, base);
    assert_eq!(rect2.width, 100);
    assert_eq!(rect2.height, 100);

    let rect3 = centered_rect(10, 10, base);
    assert_eq!(rect3.width, 10);
    assert_eq!(rect3.height, 10);
}

#[test]
fn test_centered_rect_small_base() {
    let base = Rect {
        x: 0,
        y: 0,
        width: 10,
        height: 10,
    };
    let rect = centered_rect(50, 50, base);
    assert_eq!(rect.width, 5);
    assert_eq!(rect.height, 5);
    assert_eq!(rect.x, 3);
    assert_eq!(rect.y, 3);
}

#[test]
fn test_centered_rect_edge_cases() {
    let base = Rect::new(0, 0, 100, 50);

    let rect1 = centered_rect(0, 0, base);
    assert_eq!(rect1.width, 0);
    assert_eq!(rect1.height, 0);

    let rect2 = centered_rect(200, 100, base);
    assert_eq!(rect2.width, 100);
    assert_eq!(rect2.height, 50);

    let rect3 = centered_rect(1, 1, base);
    assert_eq!(rect3.width, 1);
    assert_eq!(rect3.height, 0);

    let tiny_base = Rect::new(0, 0, 1, 1);
    let rect4 = centered_rect(50, 50, tiny_base);
    assert_eq!(rect4.width, 1);
    assert_eq!(rect4.height, 1);
}
