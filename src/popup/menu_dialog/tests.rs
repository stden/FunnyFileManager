use super::*;
use crate::{
    menu::{MenuCommand, SettingsPage},
    models::Panel,
};

fn fixture() -> (tempfile::TempDir, App) {
    let temp = tempfile::TempDir::new().unwrap();
    let panel = Panel::new(temp.path().to_owned()).unwrap();
    (temp, App::from_panels(panel.clone(), panel))
}

#[test]
fn prompt_edits_text_without_leaking_modified_shortcuts() {
    let (_temp, mut app) = fixture();
    crate::ops::menu::execute(&mut app, MenuCommand::Filter);
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
    crate::ops::menu::execute(&mut app, MenuCommand::Screens);
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
    crate::ops::menu::execute(&mut app, MenuCommand::ViewHistory);
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
    crate::ops::menu::execute(&mut app, MenuCommand::Screens);
    process(&mut app, KeyCode::Up, KeyModifiers::NONE);
    process(&mut app, KeyCode::Down, KeyModifiers::NONE);
    process(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(app.view, crate::models::ViewMode::Console);
    assert!(!app.overlay.is_active());
}
