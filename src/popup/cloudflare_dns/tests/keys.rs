//! Purpose: The keys the lists and the form do not use change nothing, and the arrows step the form
//! and its text.

use super::*;
use crossterm::event::KeyModifiers;

fn form_values(app: &App) -> (String, String, String) {
    let View::Edit(form) = &dialog(app).view else {
        panic!("the form");
    };
    (form.name.clone(), form.content.clone(), form.ttl.clone())
}

#[test]
fn a_key_is_not_taken_when_no_dialog_is_open() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    assert!(!super::super::process(
        &mut app,
        KeyCode::Up,
        KeyModifiers::NONE
    ));
}

#[test]
fn the_list_ignores_the_keys_it_does_not_use() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    open_with(&mut app, idle_dialog());
    press(&mut app, KeyCode::Char('x'));
    press(&mut app, KeyCode::Left);
    assert_eq!(dialog(&app).view.kind(), ViewKind::Browse);
    assert!(dialog(&app).busy.is_none(), "nothing was asked for");
}

#[test]
fn up_steps_the_zones_back() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    let mut panel = idle_dialog();
    panel.select_zone(1);
    open_with(&mut app, panel);
    press(&mut app, KeyCode::Up);
    assert_eq!(
        dialog(&app).selected_zone().map(|zone| zone.name.as_str()),
        Some("example.com")
    );
}

#[test]
fn the_form_steps_back_and_a_typed_character_is_taken_back_and_other_keys_change_nothing() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    open_with(&mut app, idle_dialog());
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Enter);
    let start = form_values(&app);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Up);
    press(&mut app, KeyCode::Char('x'));
    press(&mut app, KeyCode::Backspace);
    press(&mut app, KeyCode::F(5));
    assert_eq!(form_values(&app), start);
    assert_eq!(dialog(&app).view.kind(), ViewKind::Edit);
}
