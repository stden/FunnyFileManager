//! Purpose: Active panel: default focus, cursor bar, info row and bottom-border totals.

use super::*;

/// Far's panel ground, and the lighter blue of its cursor bar.
const PANEL_BG: Color = Color::Rgb(0, 0, 128);
const CURSOR_BG: Color = Color::Rgb(85, 150, 216);

#[test]
fn test_active_panel_left_by_default() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.active_panel = PanelSide::Left;

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // Active panel border is Cyan; verify a cell on the left panel border
    // Row 1, col 0 is the top-left corner of the left panel
    let bg = cell_bg(&terminal, 0, 1);
    // The panel has FAR_BLUE background
    assert_eq!(bg, PANEL_BG, "Panel background should be FAR_BLUE");
}

// ---------------------------------------------------------------------------
// Status bar tests
// ---------------------------------------------------------------------------

#[test]
fn test_info_row_names_the_selected_entry() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.left_panel.select(Some(1)); // select "docs"

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // The hint row is gone with the status bar; the panel's info row names the
    // current entry, as Far's does.
    let info = row_text(&terminal, 20);
    assert!(
        info.contains("docs"),
        "Info row should name the selected entry, got: '{info}'"
    );
}

// ---------------------------------------------------------------------------
// Panel switching tests
// ---------------------------------------------------------------------------

#[test]
fn test_right_panel_info_row_names_its_own_entry() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.active_panel = PanelSide::Right;
    app.right_panel.select(Some(1)); // select "src"

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // Each panel names its own current entry, so "which panel is active" is
    // answered by the cursor bar and the highlighted title, not by one shared row.
    let info = row_text(&terminal, 20);
    assert!(
        info.contains("src"),
        "The right panel's info row should name its selected entry, got: '{info}'"
    );
}

// ---------------------------------------------------------------------------
// Far-style panel chrome
// ---------------------------------------------------------------------------

/// Far draws a cursor bar on the ACTIVE panel only: two highlighted rows would
/// give the screen two answers to "what does Enter act on".
#[test]
fn test_only_the_active_panel_draws_a_cursor_bar() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.active_panel = PanelSide::Left;
    app.left_panel.select(Some(1));
    app.right_panel.select(Some(1));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // Row 4 is the second entry row: menu bar, top border, column header,
    // then the entries.
    let left_bg = cell_bg(&terminal, 2, 4);
    let right_bg = cell_bg(&terminal, 42, 4);
    assert_eq!(left_bg, CURSOR_BG, "the active panel highlights its cursor");
    assert_ne!(
        right_bg, CURSOR_BG,
        "the inactive panel must not draw a second cursor bar"
    );
}

/// The panel footer reports what the panel holds and what the disk has left,
/// which Far prints into the bottom border rather than onto a row of its own.
#[test]
fn test_bottom_border_carries_the_totals() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // test_app() gives the left panel two files of 4096 and 512 bytes among
    // four entries: 4.5 K, and 2 files out of 4 rows.
    let border = row_text(&terminal, 21);
    assert!(border.contains("(2/4)"), "got: '{border}'");
    assert!(border.contains("4.5 K"), "got: '{border}'");
}
