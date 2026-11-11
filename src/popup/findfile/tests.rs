use crossterm::event::{KeyCode, KeyModifiers};

use crate::models::{App, Overlay, Panel};
use crate::popup::process_popup_input;
use crate::popup::tests::key;
use std::fs;
use tempfile::TempDir;

/// Opening sets the variant; Esc takes it down and runs nothing else.
#[test]
fn open_sets_the_variant_and_esc_closes_it() {
    let mut app = App::new().expect("test setup");
    crate::ops::findfile::open(&mut app);
    assert!(matches!(app.overlay, Overlay::FindFile(_)));

    process_popup_input(&mut app, key(KeyCode::Char('x')));
    assert!(
        matches!(app.overlay, Overlay::FindFile(_)),
        "typing stays in the dialog"
    );

    process_popup_input(&mut app, key(KeyCode::Esc));
    assert!(!app.overlay.is_active());
}

#[test]
fn enter_runs_the_search_instead_of_make_folder() {
    let dir = TempDir::new().expect("dir");
    fs::write(dir.path().join("hit.txt"), "x").expect("file");
    let panel = Panel::new(dir.path().to_path_buf()).expect("panel");
    let mut app = App::from_panels(panel.clone(), panel);
    crate::ops::findfile::open(&mut app);
    process_popup_input(&mut app, key(KeyCode::Enter));
    let Overlay::MenuDialog(dialog) = &app.overlay else {
        panic!("search results, not Make folder");
    };
    let crate::models::MenuDialog::Choices { title, .. } = dialog.as_ref() else {
        panic!("choices");
    };
    assert_eq!(title, "Find file");
}

fn open_app() -> App {
    let mut app = App::new().expect("test setup");
    crate::ops::findfile::open(&mut app);
    app
}

fn field(app: &App) -> crate::models::FindFileField {
    let Overlay::FindFile(dialog) = &app.overlay else {
        panic!("find file");
    };
    dialog.field()
}

#[test]
fn tab_and_arrows_walk_the_controls() {
    use crate::models::FindFileField;
    let mut app = open_app();
    process_popup_input(&mut app, key(KeyCode::Tab));
    assert_eq!(field(&app), FindFileField::Containing);
    process_popup_input(&mut app, key(KeyCode::Down));
    assert_eq!(field(&app), FindFileField::Find);
    process_popup_input(&mut app, key(KeyCode::Right));
    assert_eq!(field(&app), FindFileField::Cancel);
    process_popup_input(&mut app, key(KeyCode::Left));
    assert_eq!(field(&app), FindFileField::Find);
    process_popup_input(&mut app, key(KeyCode::Up));
    assert_eq!(field(&app), FindFileField::Containing);
    process_popup_input(&mut app, key(KeyCode::BackTab));
    assert_eq!(field(&app), FindFileField::Mask);
}

#[test]
fn enter_on_cancel_and_f10_close_without_searching() {
    let mut app = open_app();
    process_popup_input(&mut app, key(KeyCode::BackTab));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(!app.overlay.is_active());

    let mut app = open_app();
    process_popup_input(&mut app, key(KeyCode::F(10)));
    assert!(!app.overlay.is_active());
}

#[test]
fn backspace_and_ctrl_do_not_leave_the_dialog() {
    use crossterm::event::{KeyEvent, KeyModifiers};
    let mut app = open_app();
    process_popup_input(&mut app, key(KeyCode::Backspace));
    let Overlay::FindFile(dialog) = &app.overlay else {
        panic!("still open");
    };
    assert!(dialog.mask.is_empty());
    process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
    );
    assert!(matches!(app.overlay, Overlay::FindFile(_)));
}

#[test]
fn process_returns_false_when_the_dialog_is_not_open() {
    let mut app = App::new().expect("app");
    assert!(!super::process(
        &mut app,
        KeyCode::Enter,
        KeyModifiers::empty()
    ));
}
