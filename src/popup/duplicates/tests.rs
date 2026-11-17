use crossterm::event::{KeyCode, KeyModifiers};
use std::fs;
use tempfile::TempDir;

use crate::models::{App, Duplicates, DuplicatesField, MenuDialog, Overlay, Panel};
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

fn dialog(app: &App) -> &Duplicates {
    let Overlay::Duplicates(dialog) = &app.overlay else {
        panic!("the dialog");
    };
    dialog
}

#[test]
fn typing_edits_only_the_focused_field() {
    let dir = two_copies();
    let mut app = open_on(dir.path());
    let folder = dialog(&app).folder.text().to_owned();
    press(&mut app, KeyCode::Tab, 2);
    assert_eq!(field(&app), DuplicatesField::MinBlock);
    press(&mut app, KeyCode::Char('0'), 1);
    assert_eq!(dialog(&app).min_block.text(), "60");
    press(&mut app, KeyCode::Backspace, 2);
    press(&mut app, KeyCode::Char('3'), 1);
    assert_eq!(dialog(&app).min_block.text(), "3");
    assert_eq!(dialog(&app).folder.text(), folder);
    assert_eq!(dialog(&app).mask.text(), "*");
    assert_eq!(dialog(&app).groups.text(), "50");
}

#[test]
fn left_and_right_move_the_caret_in_a_field_and_switch_between_the_buttons() {
    let dir = two_copies();
    let mut app = open_on(dir.path());
    press(&mut app, KeyCode::Tab, 2);
    press(&mut app, KeyCode::Left, 1);
    press(&mut app, KeyCode::Char('9'), 1);
    assert_eq!(field(&app), DuplicatesField::MinBlock);
    assert_eq!(dialog(&app).min_block.text(), "96", "Left moved the caret");
    press(&mut app, KeyCode::Tab, 2);
    assert_eq!(field(&app), DuplicatesField::Start);
    press(&mut app, KeyCode::Right, 1);
    assert_eq!(field(&app), DuplicatesField::Cancel);
    press(&mut app, KeyCode::Left, 1);
    assert_eq!(field(&app), DuplicatesField::Start);
    assert_eq!(dialog(&app).min_block.text(), "96");
}

#[test]
fn the_dialog_takes_no_keys_while_it_is_not_open() {
    let dir = two_copies();
    let mut app = open_on(dir.path());
    app.close_overlay();
    assert!(!super::process(&mut app, KeyCode::Esc, KeyModifiers::NONE));
    assert!(!super::process(
        &mut app,
        KeyCode::Char('x'),
        KeyModifiers::NONE
    ));
    assert!(matches!(app.overlay, Overlay::None));
}
