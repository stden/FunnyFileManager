use crossterm::event::KeyCode;
use std::fs;
use tempfile::TempDir;

use crate::models::{App, DuplicatesField, MenuDialog, Overlay, Panel};
use crate::popup::process_popup_input;
use crate::popup::tests::key;

fn open_on(folder: &std::path::Path) -> App {
    let panel = Panel::new(folder.to_path_buf()).expect("panel");
    let mut app = App::from_panels(panel.clone(), panel);
    crate::ops::duplicates::open(&mut app);
    app
}

fn field(app: &App) -> DuplicatesField {
    let Overlay::Duplicates(dialog) = &app.overlay else {
        panic!("the dialog");
    };
    dialog.field()
}

fn press(app: &mut App, code: KeyCode, times: usize) {
    for _ in 0..times {
        process_popup_input(app, key(code));
    }
}

/// Two files that share a six-line block, the second after two lines of its own.
fn two_copies() -> TempDir {
    let block = (1..=6)
        .map(|n| format!("shared {n}"))
        .collect::<Vec<_>>()
        .join("\n");
    let dir = TempDir::new().expect("dir");
    fs::write(dir.path().join("a.rs"), format!("{block}\n")).expect("first copy");
    fs::write(dir.path().join("b.rs"), format!("intro\nmore\n{block}\n")).expect("second copy");
    dir
}

#[test]
fn esc_closes_the_dialog_and_runs_nothing() {
    let dir = two_copies();
    let mut app = open_on(dir.path());
    process_popup_input(&mut app, key(KeyCode::Esc));
    assert!(!app.overlay.is_active());
}

#[test]
fn tab_and_arrows_walk_the_controls() {
    let dir = two_copies();
    let mut app = open_on(dir.path());
    press(&mut app, KeyCode::Tab, 1);
    assert_eq!(field(&app), DuplicatesField::Mask);
    press(&mut app, KeyCode::Down, 1);
    assert_eq!(field(&app), DuplicatesField::MinBlock);
    press(&mut app, KeyCode::Up, 1);
    assert_eq!(field(&app), DuplicatesField::Mask);
    press(&mut app, KeyCode::BackTab, 1);
    assert_eq!(field(&app), DuplicatesField::Folder);
}

#[test]
fn enter_on_cancel_closes_without_running() {
    let dir = two_copies();
    let mut app = open_on(dir.path());
    press(&mut app, KeyCode::Tab, 5);
    assert_eq!(field(&app), DuplicatesField::Cancel);
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(!app.overlay.is_active());
}

#[test]
fn enter_on_start_runs_the_search_and_lists_its_copies() {
    let dir = two_copies();
    let mut app = open_on(dir.path());
    press(&mut app, KeyCode::Tab, 4);
    assert_eq!(field(&app), DuplicatesField::Start);
    process_popup_input(&mut app, key(KeyCode::Enter));
    let Overlay::MenuDialog(list) = &app.overlay else {
        panic!("the list of repeats");
    };
    let MenuDialog::Choices { title, rows, .. } = list.as_ref() else {
        panic!("choices");
    };
    assert_eq!(title, "Find duplicate blocks");
    assert_eq!(rows.len(), 2);
}

#[test]
fn esc_on_the_list_closes_it() {
    let dir = two_copies();
    let mut app = open_on(dir.path());
    press(&mut app, KeyCode::Tab, 4);
    process_popup_input(&mut app, key(KeyCode::Enter));
    press(&mut app, KeyCode::Esc, 1);
    assert!(!app.overlay.is_active());
}

#[test]
fn enter_on_a_row_opens_the_file_with_the_cursor_on_the_block() {
    let dir = two_copies();
    let mut app = open_on(dir.path());
    press(&mut app, KeyCode::Tab, 4);
    process_popup_input(&mut app, key(KeyCode::Enter));
    press(&mut app, KeyCode::Down, 1);
    process_popup_input(&mut app, key(KeyCode::Enter));
    let Overlay::Editor(editor) = &app.overlay else {
        panic!("the editor, on the second copy");
    };
    assert_eq!(editor.cursor_line, 2, "the block starts on line 3");
    assert!(editor.lines.get(2).is_some_and(|line| line == "shared 1"));
}
