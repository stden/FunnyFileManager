//! Unit tests: where a drop-down goes and what a click on it hits — under
//! its own heading, pulled back on screen, nothing on a screen too small for
//! the box, and no row outside it, below it or on a separator.

use super::geometry::drop_down_area;
use super::tests::{bar, screen};
use super::*;
use crate::menu::Menu;

// ─── geometry ───────────────────────────────────────────────────────────────

#[test]
fn a_drop_down_sits_under_its_own_heading() {
    let menu = Menu::open(Heading::Files);
    let area = drop_down_area(&menu, bar(120), screen(120, 40)).expect("room for it");
    assert_eq!(area.y, 1, "immediately under the bar");
    assert!(area.height >= 3);
}

/// Pulled left rather than drawn off the edge: a box that starts past the last
/// column is invisible, which reads as the menu having failed to open.
#[test]
fn a_drop_down_near_the_right_edge_is_pulled_back_on_screen() {
    let menu = Menu::open(Heading::Right);
    let width = 60;
    let area = drop_down_area(&menu, bar(width), screen(width, 40)).expect("room for it");
    assert!(
        area.x + area.width <= width,
        "the box ends on screen: {area:?}"
    );
}

/// A screen with no room for the box produces nothing, and the mouse agrees
/// with the renderer about that.
#[test]
fn a_screen_too_small_for_the_box_gets_none() {
    let menu = Menu::open(Heading::Files);
    assert_eq!(
        drop_down_area(&menu, bar(3), screen(3, 40)),
        None,
        "too narrow"
    );
    assert_eq!(
        drop_down_area(&menu, bar(80), screen(80, 3)),
        None,
        "too short"
    );
    assert_eq!(
        menu_row_at(&menu, screen(3, 40), 1, 2),
        None,
        "and no row can be hit in a box that is not there"
    );
}

// ─── which row a click lands on ─────────────────────────────────────────────

#[test]
fn a_click_outside_the_box_hits_no_row() {
    let menu = Menu::open(Heading::Files);
    let full = screen(120, 40);
    let area = drop_down_area(&menu, bar(120), full).expect("room for it");

    assert_eq!(
        menu_row_at(&menu, full, area.x, area.y + 1),
        None,
        "on the frame"
    );
    assert_eq!(
        menu_row_at(&menu, full, area.x + area.width, area.y + 1),
        None,
        "past the right edge"
    );
    assert_eq!(
        menu_row_at(&menu, full, area.x + 2, area.y),
        None,
        "on the top border, which is above every row"
    );
}

/// A click below the last row hits nothing, rather than the last row again.
#[test]
fn a_click_below_the_rows_hits_nothing() {
    let menu = Menu::open(Heading::Files);
    let full = screen(120, 40);
    let area = drop_down_area(&menu, bar(120), full).expect("room for it");
    let below = area.y + area.height + 5;
    assert_eq!(menu_row_at(&menu, full, area.x + 2, below), None);
}

/// A separator is not a row anything can be chosen on, so a click on one has
/// to be nothing rather than the row above or below it.
#[test]
fn a_click_on_a_separator_chooses_nothing() {
    let menu = Menu::open(Heading::Files);
    let full = screen(120, 40);
    let area = drop_down_area(&menu, bar(120), full).expect("room for it");
    let separator = menu
        .entries()
        .iter()
        .position(|entry| !entry.is_selectable())
        .expect("Files has separators and greyed rows");
    let row = area.y + 1 + u16::try_from(separator).expect("small index");
    assert_eq!(menu_row_at(&menu, full, area.x + 2, row), None);
}

// ─── the renderer itself, when there is no room for the box ────────────────

/// [`render_drop_down`] asks the same question the mouse does, and has to
/// give up the same way: draw nothing rather than panic on a rectangle that
/// [`drop_down_area`] refused to hand back.
#[test]
fn render_drop_down_draws_nothing_when_the_screen_has_no_room() {
    use crate::models::{App, Overlay};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::Menu(Menu::open(Heading::Files)));

    // 3x3 is too narrow for even the tightest drop-down, so
    // `drop_down_area` returns `None` and the renderer must simply return.
    let backend = TestBackend::new(3, 3);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            render_drop_down(f, &app, Rect::new(0, 0, 3, 1));
        })
        .unwrap();
}
