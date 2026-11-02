//! Unit tests: the Things dialog's keys, from typing a filter to importing, and
//! proof that every key the dialog is given is taken by it and never reaches
//! the panels behind.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::models::things::{ThingsCatalog, ThingsDialog, ThingsState};
use crate::models::{App, Overlay, Panel};
use crate::popup::process_popup_input;
use crate::status::Status;
use crate::things::{Listing, Machine};

fn app_with(state: ThingsState) -> App {
    let panel = Panel::from_entries(std::env::temp_dir(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    app.set_overlay(Overlay::Things(Box::new(ThingsDialog::new(state))));
    app
}

/// The dialog open on machines named `names`, sorted as a read sorts them.
fn ready(names: &[String]) -> App {
    let machines = names
        .iter()
        .map(|name| Machine {
            id: format!("id-{name}"),
            name: name.clone(),
            hostname: format!("{name}.example.com"),
            ..Machine::default()
        })
        .collect();
    app_with(ThingsState::Ready(ThingsCatalog::new(Listing::new(
        machines, None,
    ))))
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|name| (*name).to_owned()).collect()
}

fn catalog(app: &App) -> &ThingsCatalog {
    let Overlay::Things(dialog) = &app.overlay else {
        panic!("the dialog is open");
    };
    let ThingsState::Ready(catalog) = &dialog.state else {
        panic!("the machines are shown");
    };
    catalog
}

fn press(app: &mut App, code: KeyCode) {
    press_with(app, code, KeyModifiers::empty());
}

fn press_with(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    assert!(
        process_popup_input(app, KeyEvent::new(code, modifiers)),
        "{code:?} is not taken by the dialog"
    );
}

#[test]
fn typing_filters_the_list_and_backspace_edits_the_filter() {
    let mut app = ready(&names(&["web-01", "db-01"]));
    press(&mut app, KeyCode::Char('w'));
    press(&mut app, KeyCode::Char('E'));
    assert_eq!(catalog(&app).filter(), "wE");
    assert_eq!(catalog(&app).shown().len(), 1);
    press(&mut app, KeyCode::Backspace);
    assert_eq!(catalog(&app).filter(), "w");
}

#[test]
fn space_marks_the_row_under_the_cursor_and_is_not_typed_into_the_filter() {
    let mut app = ready(&names(&["db-01", "web-01"]));
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char(' '));
    assert_eq!(catalog(&app).filter(), "");
    let marked: Vec<bool> = catalog(&app)
        .shown()
        .iter()
        .map(|(marked, _)| *marked)
        .collect();
    assert_eq!(marked, [false, true]);
}

#[test]
fn the_arrows_and_paging_keys_move_the_cursor() {
    let fixtures: Vec<String> = (0..25).map(|index| format!("m{index:02}")).collect();
    let mut app = ready(&fixtures);
    press(&mut app, KeyCode::End);
    assert_eq!(catalog(&app).cursor_index(), 24);
    press(&mut app, KeyCode::PageUp);
    assert_eq!(catalog(&app).cursor_index(), 14);
    press(&mut app, KeyCode::Home);
    assert_eq!(catalog(&app).cursor_index(), 0);
    press(&mut app, KeyCode::PageDown);
    assert_eq!(catalog(&app).cursor_index(), 10);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Up);
    press(&mut app, KeyCode::Up);
    assert_eq!(catalog(&app).cursor_index(), 9);
}

#[test]
fn enter_imports_the_shown_machines_and_closes_the_dialog() {
    let mut app = ready(&names(&["web-01"]));
    press(&mut app, KeyCode::Enter);
    assert!(!app.overlay.is_active());
    assert_eq!(
        app.status,
        Status::CommandOutput("Things: 1 imported, 0 skipped".to_owned())
    );
}

#[test]
fn esc_and_f10_close_the_dialog() {
    for code in [KeyCode::Esc, KeyCode::F(10)] {
        let mut app = ready(&names(&["web-01"]));
        press(&mut app, code);
        assert!(!app.overlay.is_active(), "{code:?}");
    }
}

#[test]
fn while_the_read_is_out_only_enter_and_esc_do_anything() {
    let mut app = app_with(ThingsState::Loading);
    press(&mut app, KeyCode::Char('x'));
    press(&mut app, KeyCode::Down);
    assert!(matches!(
        &app.overlay,
        Overlay::Things(dialog) if matches!(dialog.state, ThingsState::Loading)
    ));
    press(&mut app, KeyCode::Enter);
    assert!(!app.overlay.is_active());
}

#[test]
fn every_key_is_taken_so_typing_and_function_keys_never_reach_the_panels() {
    let mut app = ready(&names(&["web-01"]));
    for code in [
        KeyCode::Char('q'),
        KeyCode::F(3),
        KeyCode::Tab,
        KeyCode::Left,
    ] {
        press(&mut app, code);
    }
    press_with(&mut app, KeyCode::Char('w'), KeyModifiers::CONTROL);
    assert!(matches!(app.overlay, Overlay::Things(_)));
    assert_eq!(catalog(&app).filter(), "q");
    assert!(app.lifecycle.is_running());
}
