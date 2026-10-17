use super::*;
use crate::{
    models::Panel,
    path_env::{Scope, Snapshot, Validation},
};
use tempfile::TempDir;

fn fixture() -> (TempDir, App) {
    let temp = TempDir::new().expect("tempdir");
    let panel = Panel::new(temp.path().to_owned()).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    let snapshot = Snapshot {
        value: String::new(),
        revision: String::new(),
        location: "fixture only".to_owned(),
    };
    app.set_overlay(Overlay::PathEditor(Box::new(PathEditor::new(
        Scope::User,
        snapshot,
    ))));
    (temp, app)
}

fn dialog(app: &App) -> &PathEditor {
    let Overlay::PathEditor(editor) = &app.overlay else {
        panic!("PATH dialog");
    };
    editor
}

fn key(app: &mut App, code: KeyCode) {
    assert!(process(app, code, KeyModifiers::NONE));
}

fn add(app: &mut App, text: &str) {
    key(app, KeyCode::Insert);
    for ch in text.chars() {
        key(app, KeyCode::Char(ch));
    }
    key(app, KeyCode::Enter);
}

#[test]
fn add_edit_reorder_remove_and_discard_stay_inside_the_draft() {
    let (temp, mut app) = fixture();
    add(&mut app, &temp.path().to_string_lossy());
    assert_eq!(dialog(&app).entries[0].validation, Validation::Directory);
    add(&mut app, &temp.path().join("missing").to_string_lossy());
    assert_eq!(dialog(&app).entries[1].validation, Validation::Missing);
    process(&mut app, KeyCode::Up, KeyModifiers::CONTROL);
    assert_eq!(dialog(&app).entries[0].validation, Validation::Missing);
    key(&mut app, KeyCode::Delete);
    key(&mut app, KeyCode::F(4));
    key(&mut app, KeyCode::Esc);
    key(&mut app, KeyCode::F(2));
    assert!(matches!(dialog(&app).mode, Mode::ConfirmSave));
    key(&mut app, KeyCode::Esc);
    key(&mut app, KeyCode::Esc);
    assert!(matches!(dialog(&app).mode, Mode::ConfirmDiscard));
    key(&mut app, KeyCode::Enter);
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn invalid_entries_and_multiple_directories_cannot_be_saved() {
    let (temp, mut app) = fixture();
    add(&mut app, &temp.path().join("missing").to_string_lossy());
    key(&mut app, KeyCode::F(2));
    key(&mut app, KeyCode::Enter);
    assert!(dialog(&app).message.contains("invalid paths"));
    add(
        &mut app,
        &format!("first{}second", crate::path_env::separator()),
    );
    assert!(dialog(&app).message.contains("one directory"));
    assert!(matches!(dialog(&app).mode, Mode::Edit { .. }));
}

#[test]
fn recheck_recognizes_a_directory_created_while_the_editor_is_open() {
    let (temp, mut app) = fixture();
    let created = temp.path().join("created-later");
    add(&mut app, &temp.path().to_string_lossy());
    add(&mut app, &created.to_string_lossy());
    assert_eq!(dialog(&app).entries[1].validation, Validation::Missing);
    std::fs::create_dir(&created).expect("new directory");
    key(&mut app, KeyCode::F(5));
    assert_eq!(dialog(&app).entries[1].validation, Validation::Directory);
    key(&mut app, KeyCode::Home);
    assert_eq!(dialog(&app).selected, 0);
    process(&mut app, KeyCode::Down, KeyModifiers::CONTROL);
    assert_eq!(
        dialog(&app).entries[1].text,
        temp.path().display().to_string()
    );
    key(&mut app, KeyCode::End);
    assert_eq!(dialog(&app).selected, 1);
}

#[test]
fn confirmations_ignore_unrelated_keys_and_esc_returns_to_the_draft() {
    let (temp, mut app) = fixture();
    add(&mut app, &temp.path().to_string_lossy());

    key(&mut app, KeyCode::F(2));
    key(&mut app, KeyCode::Char('x'));
    assert!(matches!(dialog(&app).mode, Mode::ConfirmSave));
    key(&mut app, KeyCode::Esc);
    assert!(matches!(dialog(&app).mode, Mode::Browse));

    key(&mut app, KeyCode::Esc);
    assert!(matches!(dialog(&app).mode, Mode::ConfirmDiscard));
    key(&mut app, KeyCode::Char('x'));
    assert!(matches!(dialog(&app).mode, Mode::ConfirmDiscard));
    key(&mut app, KeyCode::Esc);
    assert!(matches!(dialog(&app).mode, Mode::Browse));

    key(&mut app, KeyCode::Char('x'));
    assert!(matches!(dialog(&app).mode, Mode::Browse));
    assert_eq!(dialog(&app).entries.len(), 1, "the draft survives");
}

#[test]
fn keys_are_left_for_others_when_the_path_dialog_is_not_open() {
    let (_temp, mut app) = fixture();
    app.close_overlay();
    assert!(!process(&mut app, KeyCode::Esc, KeyModifiers::NONE));
}
