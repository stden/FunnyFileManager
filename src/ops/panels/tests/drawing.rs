//! Purpose: What a hidden panel leaves on screen: the freed half and the full width.

use super::*;

/// A click on the half the hidden panel used to have lands on the panel
/// that now fills it, never on the hidden one.
#[test]
fn a_click_on_the_freed_half_lands_on_the_visible_panel() {
    let mut app = app();
    let screen = ratatui::layout::Rect::new(0, 0, 80, 24);
    toggle(&mut app, PanelSide::Left);
    let target = hit(screen, 10, 3, FKEY_COUNT, app.panels_on_screen());
    assert!(
        matches!(
            target,
            Target::Panel(PanelSide::Right)
                | Target::PanelRow {
                    side: PanelSide::Right,
                    ..
                }
        ),
        "{target:?}"
    );
}

/// Rendered through the real layout: the visible panel's title is centred on
/// the whole width, and the hidden panel's path is nowhere on screen.
#[test]
fn the_visible_panel_takes_the_full_width_on_screen() {
    let root = tempfile::tempdir().expect("temp dir");
    let (left_dir, right_dir) = two_directories(root.path());
    let mut app = app();
    app.left_panel.navigate_to(left_dir).expect("left panel");
    app.right_panel.navigate_to(right_dir).expect("right panel");

    let both = draw(&mut app);
    let title_row = both.lines().nth(1).expect("title row");
    let half_column = title_row.find("left_side_dir").expect("left title");
    assert!(half_column < 40, "centred in the left half: {title_row:?}");
    assert!(both.contains("right_side_dir"), "{both}");

    toggle(&mut app, PanelSide::Right);
    let one = draw(&mut app);
    let title_row = one.lines().nth(1).expect("title row");
    let full_column = title_row.find("left_side_dir").expect("left title");
    assert!(
        full_column > half_column,
        "centred on the full width: {title_row:?}"
    );
    assert!(
        title_row.trim_end().len() >= 79,
        "the frame spans the screen: {title_row:?}"
    );
    assert!(
        !one.contains("right_side_dir"),
        "the hidden panel is off screen: {one}"
    );
}
