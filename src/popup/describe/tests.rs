//! Unit tests: the keys of the Describe files dialog, over a real directory.

use std::fs;
use std::path::Path;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

use crate::models::{App, FileName, Overlay, Panel};
use crate::popup::process_popup_input;
use crate::popup::tests::key;

fn app_over(dir: &Path) -> App {
    fs::write(dir.join("a.txt"), "").expect("a.txt");
    fs::write(dir.join("b.txt"), "").expect("b.txt");
    let mut app = App::with_panels(
        Panel::new(dir.to_path_buf()).expect("left"),
        Panel::new(dir.to_path_buf()).expect("right"),
    );
    assert!(app.get_active_panel().select_named("a.txt"));
    app
}

fn type_text(app: &mut App, text: &str) {
    for ch in text.chars() {
        process_popup_input(app, key(KeyCode::Char(ch)));
    }
}

fn prompt(app: &App) -> &crate::models::DescribePrompt {
    match &app.overlay {
        Overlay::Describe(prompt) => prompt,
        _ => panic!("expected the describe prompt to be open"),
    }
}

fn description_file(dir: &Path) -> String {
    fs::read_to_string(dir.join("Descript.ion")).unwrap_or_default()
}

/// Opening sets the variant; Esc takes it down and runs nothing else.
#[test]
fn open_sets_the_variant_and_esc_closes_it() {
    let temp = TempDir::new().expect("temp");
    let mut app = app_over(temp.path());
    crate::ops::describe::open(&mut app);
    assert!(matches!(app.overlay, Overlay::Describe(_)));

    process_popup_input(&mut app, key(KeyCode::F(1)));
    assert!(
        matches!(app.overlay, Overlay::Describe(_)),
        "other keys are consumed"
    );

    process_popup_input(&mut app, key(KeyCode::Esc));
    assert!(!app.overlay.is_active());
    assert_eq!(description_file(temp.path()), "", "Esc writes nothing");
}

#[test]
fn typing_and_backspace_edit_the_text() {
    let temp = TempDir::new().expect("temp");
    let mut app = app_over(temp.path());
    crate::ops::describe::open(&mut app);
    type_text(&mut app, "Notes!");
    process_popup_input(&mut app, key(KeyCode::Backspace));
    assert_eq!(prompt(&app).text(), "Notes");

    let ctrl_x = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL);
    process_popup_input(&mut app, ctrl_x);
    assert_eq!(prompt(&app).text(), "Notes", "a control chord is not text");
}

#[test]
fn enter_on_ok_writes_the_description() {
    let temp = TempDir::new().expect("temp");
    let mut app = app_over(temp.path());
    crate::ops::describe::open(&mut app);
    type_text(&mut app, "First file");
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(!app.overlay.is_active());
    assert_eq!(description_file(temp.path()), "a.txt First file\n");
}

#[test]
fn f10_closes_the_dialog_without_writing() {
    let temp = TempDir::new().expect("temp");
    let mut app = app_over(temp.path());
    crate::ops::describe::open(&mut app);
    type_text(&mut app, "Lost");
    process_popup_input(&mut app, key(KeyCode::F(10)));
    assert!(!app.overlay.is_active());
    assert_eq!(description_file(temp.path()), "", "F10 writes nothing");
}

#[test]
fn tab_moves_to_cancel_and_enter_there_writes_nothing() {
    let temp = TempDir::new().expect("temp");
    let mut app = app_over(temp.path());
    crate::ops::describe::open(&mut app);
    type_text(&mut app, "Lost");
    process_popup_input(&mut app, key(KeyCode::Tab));
    assert!(prompt(&app).on_cancel());
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(!app.overlay.is_active());
    assert_eq!(description_file(temp.path()), "");
}

/// Far's field is an edit control: Left goes back to the typo, not to
/// Cancel, and Enter afterwards still writes what was typed.
#[test]
fn left_and_right_move_the_caret_inside_the_text() {
    let temp = TempDir::new().expect("temp");
    let mut app = app_over(temp.path());
    crate::ops::describe::open(&mut app);
    type_text(&mut app, "Frst");
    process_popup_input(&mut app, key(KeyCode::Left));
    process_popup_input(&mut app, key(KeyCode::Left));
    process_popup_input(&mut app, key(KeyCode::Left));
    assert!(!prompt(&app).on_cancel(), "Left edits, it does not cancel");
    type_text(&mut app, "i");
    assert_eq!(prompt(&app).text(), "First");
    process_popup_input(&mut app, key(KeyCode::Right));
    process_popup_input(&mut app, key(KeyCode::Delete));
    assert_eq!(
        prompt(&app).text(),
        "Firt",
        "Delete takes the char after the caret"
    );
    process_popup_input(&mut app, key(KeyCode::Home));
    process_popup_input(&mut app, key(KeyCode::Delete));
    process_popup_input(&mut app, key(KeyCode::End));
    type_text(&mut app, "!");
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(!app.overlay.is_active());
    assert_eq!(description_file(temp.path()), "a.txt irt!\n");
}

/// On a button, Left and Right walk the buttons as Far's do; Tab is the way
/// from the field to the buttons.
#[test]
fn on_a_button_left_and_right_walk_between_ok_and_cancel() {
    let temp = TempDir::new().expect("temp");
    let mut app = app_over(temp.path());
    crate::ops::describe::open(&mut app);
    type_text(&mut app, "Kept");
    process_popup_input(&mut app, key(KeyCode::Tab));
    assert!(prompt(&app).on_cancel());
    process_popup_input(&mut app, key(KeyCode::Left));
    assert!(!prompt(&app).on_cancel(), "Left from Cancel lands on OK");
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert_eq!(description_file(temp.path()), "a.txt Kept\n");
}

#[test]
fn esc_in_the_middle_of_a_queue_drops_the_rest() {
    let temp = TempDir::new().expect("temp");
    let mut app = app_over(temp.path());
    let panel = app.get_active_panel();
    panel.marked.insert(FileName::new("a.txt"));
    panel.marked.insert(FileName::new("b.txt"));
    crate::ops::describe::open(&mut app);
    type_text(&mut app, "One");
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert_eq!(prompt(&app).name(), "b.txt");
    process_popup_input(&mut app, key(KeyCode::Esc));
    assert!(!app.overlay.is_active());
    assert_eq!(description_file(temp.path()), "a.txt One\n");
    let marked: Vec<&str> = app
        .get_active_panel()
        .marked
        .iter()
        .map(FileName::as_str)
        .collect();
    assert_eq!(
        marked,
        ["b.txt"],
        "the described file is unmarked, the rest stay"
    );
}
