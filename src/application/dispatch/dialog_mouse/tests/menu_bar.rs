//! Purpose: clicks on the F9 menu bar: away from the drop-down, and on a panel's heading.

use super::*;

#[test]
fn clicking_away_closes_the_menu() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let menu = Menu::open(Heading::Files);
    app.set_menu(menu);
    app.dialog_mouse.screen = ratatui::layout::Rect::new(0, 0, 80, 24);
    click_in_menu(&mut app, menu, 0, 10);
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn clicking_a_panel_heading_makes_that_panel_active() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.active_panel = PanelSide::Right;
    let menu = Menu::open(Heading::Files);
    app.set_menu(menu);
    let screen = ratatui::layout::Rect::new(0, 0, 80, 24);
    app.dialog_mouse.screen = screen;
    let column = (0..80)
        .find(|&col| crate::ui::heading_at(screen, col) == Some(Heading::Left))
        .expect("Left heading is on the bar");
    click_in_menu(&mut app, menu, column, screen.y);
    assert_eq!(app.active_panel, PanelSide::Left);
    assert!(matches!(
        app.overlay,
        Overlay::Menu(open) if open.heading == Heading::Left
    ));
}

/// A hidden panel's heading still opens its drop-down — that is where its
/// "Panel On/Off" row lives — but the cursor stays on the panel that is on
/// screen, as it does for the keyboard walk.
#[test]
fn clicking_a_hidden_panels_heading_keeps_the_cursor_on_the_visible_one() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.active_panel = PanelSide::Left;
    crate::ops::panels::toggle(&mut app, PanelSide::Right);
    let menu = Menu::open(Heading::Files);
    app.set_menu(menu);
    let screen = ratatui::layout::Rect::new(0, 0, 80, 24);
    app.dialog_mouse.screen = screen;
    let column = (0..80)
        .find(|&col| crate::ui::heading_at(screen, col) == Some(Heading::Right))
        .expect("Right heading is on the bar");
    click_in_menu(&mut app, menu, column, screen.y);
    assert_eq!(app.active_panel, PanelSide::Left);
    assert!(matches!(
        app.overlay,
        Overlay::Menu(open) if open.heading == Heading::Right
    ));
}
