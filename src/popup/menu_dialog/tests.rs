use super::*;
use crate::{
    action::ToolCommand,
    menu::{MenuCommand, SettingsPage},
    models::Panel,
};

fn open_tool(app: &mut App, command: ToolCommand) {
    crate::ops::menu::execute_tool(app, command).expect("the tool opens");
}

fn fixture() -> (tempfile::TempDir, App) {
    let temp = tempfile::TempDir::new().unwrap();
    let panel = Panel::new(temp.path().to_owned()).unwrap();
    (temp, App::from_panels(panel.clone(), panel))
}

#[test]
fn prompt_edits_text_without_leaking_modified_shortcuts() {
    let (_temp, mut app) = fixture();
    open_tool(&mut app, ToolCommand::Filter);
    for key in [
        KeyCode::Char('a'),
        KeyCode::Char('c'),
        KeyCode::Left,
        KeyCode::Char('b'),
        KeyCode::Right,
        KeyCode::Backspace,
        KeyCode::Home,
        KeyCode::Delete,
        KeyCode::End,
        KeyCode::Tab,
    ] {
        assert!(process(&mut app, key, KeyModifiers::NONE));
    }
    process(&mut app, KeyCode::Char('x'), KeyModifiers::CONTROL);
    process(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(app.left_panel.options.filter, "b");
    assert!(!app.overlay.is_active());
}

#[test]
fn invalid_input_keeps_the_prompt_open_and_cancel_setting_is_respected() {
    let (_temp, mut app) = fixture();
    crate::ops::menu::execute(&mut app, MenuCommand::Settings(SettingsPage::Editor));
    process(&mut app, KeyCode::Backspace, KeyModifiers::NONE);
    process(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert!(app.overlay.is_active());
    assert!(matches!(app.status, crate::status::Status::Error(_)));
    app.settings.set(SettingsPage::Dialog, "off").unwrap();
    process(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    assert!(app.overlay.is_active());
    process(&mut app, KeyCode::F(10), KeyModifiers::NONE);
    assert!(!app.overlay.is_active());
}

#[test]
fn choices_navigation_clamps_and_enter_applies_the_highlighted_row() {
    let (_temp, mut app) = fixture();
    open_tool(&mut app, ToolCommand::Screens);
    for key in [
        KeyCode::Up,
        KeyCode::PageDown,
        KeyCode::PageUp,
        KeyCode::End,
        KeyCode::Home,
        KeyCode::Down,
        KeyCode::Tab,
    ] {
        process(&mut app, key, KeyModifiers::NONE);
    }
    process(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(app.view, crate::models::ViewMode::Console);
    open_tool(&mut app, ToolCommand::ViewHistory);
    process(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert!(app.overlay.is_active());
    process(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    assert!(!app.overlay.is_active());
    assert!(!process(&mut app, KeyCode::Down, KeyModifiers::NONE));
    assert!(accept(&mut app).is_ok());
}

#[test]
fn menu_wrap_setting_controls_navigation_at_both_boundaries() {
    let (_temp, mut app) = fixture();
    app.settings.set(SettingsPage::Menu, "off").unwrap();
    let mut menu = crate::menu::Menu::open(crate::menu::Heading::Options);
    app.set_menu(menu);
    crate::popup::process_popup_input(
        &mut app,
        crossterm::event::KeyEvent::new(KeyCode::Up, KeyModifiers::NONE),
    );
    assert!(matches!(app.overlay, Overlay::Menu(current) if current.row == 0));
    menu.row = menu.entries().len() - 1;
    app.set_menu(menu);
    crate::popup::process_popup_input(
        &mut app,
        crossterm::event::KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
    );
    assert!(matches!(app.overlay, Overlay::Menu(current) if current.row == menu.row));
    app.settings.set(SettingsPage::Menu, "on").unwrap();
    crate::popup::process_popup_input(
        &mut app,
        crossterm::event::KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
    );
    assert!(matches!(app.overlay, Overlay::Menu(current) if current.row == 0));
}

#[test]
fn screen_choices_navigate_and_open_the_requested_screen() {
    let (_temp, mut app) = fixture();
    open_tool(&mut app, ToolCommand::Screens);
    process(&mut app, KeyCode::Up, KeyModifiers::NONE);
    process(&mut app, KeyCode::Down, KeyModifiers::NONE);
    process(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(app.view, crate::models::ViewMode::Console);
    assert!(!app.overlay.is_active());
}

/// Where `navigate` leaves a cursor at `from` in a list `length` long.
fn navigated(from: usize, length: usize, code: KeyCode) -> usize {
    let mut selected = from;
    navigate(&mut selected, length, code);
    selected
}

/// A page is `MENU_PAGE` rows, and neither a page nor a step leaves the list.
#[test]
fn navigate_pages_by_menu_page_and_stops_at_both_ends() {
    assert_eq!(navigated(0, 30, KeyCode::PageDown), MENU_PAGE);
    assert_eq!(navigated(MENU_PAGE + 1, 30, KeyCode::PageUp), 1);
    assert_eq!(navigated(25, 30, KeyCode::PageDown), 29);
    assert_eq!(navigated(3, 30, KeyCode::PageUp), 0);
    assert_eq!(navigated(29, 30, KeyCode::Down), 29);
    assert_eq!(navigated(0, 30, KeyCode::Up), 0);
    assert_eq!(navigated(7, 30, KeyCode::Home), 0);
    assert_eq!(navigated(7, 30, KeyCode::End), 29);
    assert_eq!(navigated(7, 30, KeyCode::Tab), 7, "other keys leave it");
}

/// An empty list has one place for the cursor: the top.
#[test]
fn navigate_in_an_empty_list_stays_at_the_top() {
    for code in [KeyCode::Down, KeyCode::PageDown, KeyCode::End] {
        assert_eq!(navigated(0, 0, code), 0, "{code:?}");
    }
}
