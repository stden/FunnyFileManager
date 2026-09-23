//! Unit tests: the menu bar's geometry and its label markers.
//!
//! [`drop_down_area`] is the one place that decides where a drop-down
//! goes, and both the renderer and the mouse ask it — so its refusals matter
//! as much as its answers. A screen too small for the box has to produce
//! nothing rather than a rectangle off the edge.

use ratatui::layout::Rect;

use super::*;
use crate::menu::{Heading, Menu};

fn screen(width: u16, height: u16) -> Rect {
    Rect::new(0, 0, width, height)
}

fn bar(width: u16) -> Rect {
    Rect::new(0, 0, width, 1)
}

#[test]
fn dropdown_keeps_far_colors_when_panel_highlighting_is_disabled() {
    let temp = tempfile::TempDir::new().unwrap();
    let panel = crate::models::Panel::new(temp.path().to_owned()).unwrap();
    let mut app = App::from_panels(panel.clone(), panel);
    app.settings
        .set(crate::menu::SettingsPage::Highlighting, "off")
        .unwrap();
    let menu = Menu::open(Heading::Files);
    app.set_menu(menu);
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40)).unwrap();
    terminal
        .draw(|frame| crate::ui::ui(frame, &mut app))
        .unwrap();
    let area = drop_down_area(&menu, bar(120), screen(120, 40)).unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(area.x, area.y)].fg, palette::MENU_SELECTED_FG);
    assert_eq!(buffer[(area.x, area.y)].bg, palette::MENU_BG);
    assert_eq!(
        buffer[(area.x + 1, area.y + 1)].fg,
        palette::MENU_SELECTED_FG
    );
    assert_eq!(
        buffer[(area.x + 1, area.y + 1)].bg,
        palette::MENU_SELECTED_BG
    );
    assert_eq!(
        buffer[(area.x + 1, area.y + 2)].fg,
        palette::MENU_SELECTED_FG
    );
    assert_eq!(buffer[(area.x + 1, area.y + 2)].bg, palette::MENU_BG);
}

#[test]
fn dropdown_casts_a_shadow_over_the_existing_background() {
    let temp = tempfile::TempDir::new().unwrap();
    let panel = crate::models::Panel::new(temp.path().to_owned()).unwrap();
    let mut app = App::from_panels(panel.clone(), panel);
    let menu = Menu::open(Heading::Files);
    app.set_menu(menu);
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40)).unwrap();
    terminal
        .draw(|frame| {
            frame.render_widget(
                Paragraph::new("background")
                    .style(Style::default().bg(ratatui::style::Color::Rgb(80, 80, 80))),
                frame.area(),
            );
            render_drop_down(frame, &app, bar(120));
        })
        .unwrap();
    let area = drop_down_area(&menu, bar(120), screen(120, 40)).unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(
        buffer[(area.right(), area.y + 1)].bg,
        ratatui::style::Color::Rgb(30, 30, 30)
    );
    assert_eq!(
        buffer[(area.right() + 2, area.y + 1)].bg,
        ratatui::style::Color::Rgb(80, 80, 80)
    );
}

// ─── labels ─────────────────────────────────────────────────────────────────

/// The text of a label's spans, joined back up. A renderer that dropped or
/// duplicated a character would show it here.
fn drawn(label: &str) -> String {
    hotkey_spans_in(label, Style::default(), true)
        .iter()
        .map(|span| span.content.as_ref())
        .collect()
}

#[test]
fn a_label_with_no_marker_is_drawn_whole() {
    assert_eq!(drawn("No marker here"), "No marker here");
    assert_eq!(
        hotkey_spans_in("No marker", Style::default(), true).len(),
        1
    );
}

/// A marker with nothing after it keeps the text before it rather than
/// panicking on the missing letter.
#[test]
fn a_dangling_marker_is_dropped_and_the_rest_survives() {
    assert_eq!(drawn("Dangling &"), "Dangling ");
}

#[test]
fn a_marked_label_loses_the_marker_and_keeps_every_letter() {
    assert_eq!(drawn("&Delete"), "Delete");
    assert_eq!(drawn("Find pre&vious"), "Find previous");
    assert!(
        hotkey_spans_in("&Delete", Style::default(), true).len() > 1,
        "the hotkey letter is its own span, so it can be coloured"
    );
}

/// Unaccented is how the selected row is drawn: the same characters, one
/// span fewer worth of colour.
#[test]
fn an_unaccented_label_still_reads_the_same() {
    let accented = drawn("&Delete");
    let plain: String = hotkey_spans_in("&Delete", Style::default(), false)
        .iter()
        .map(|span| span.content.as_ref())
        .collect();
    assert_eq!(plain, accented);
}

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
