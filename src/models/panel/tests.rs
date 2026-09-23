//! Unit tests: the keys of the Extract files dialog — typing edits the
//! destination, Tab walks the controls, Enter extracts, Esc cancels.

use std::fs;
use std::path::Path;
use std::process::Command;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

use crate::models::{App, ExtractField, ExtractPrompt, Overlay, Panel, PanelSide};
use crate::popup::process_popup_input;
use crate::popup::tests::key;

fn pack(root: &Path, name: &str) {
    fs::create_dir_all(root.join("payload")).expect("payload");
    fs::write(root.join("payload/hello.txt"), "hello").expect("file");
    let status = Command::new("tar")
        .arg("-caf")
        .arg(root.join(name))
        .arg("payload")
        .current_dir(root)
        .status()
        .expect("tar runs");
    assert!(status.success());
    fs::remove_dir_all(root.join("payload")).expect("payload removed");
}

fn app_with_archive(left: &Path, right: &Path) -> App {
    let mut app = App::new().expect("test app");
    app.left_panel = Panel::new(left.to_path_buf()).expect("left");
    app.right_panel = Panel::new(right.to_path_buf()).expect("right");
    app.active_panel = PanelSide::Left;
    let index = app
        .left_panel
        .entries
        .iter()
        .position(|entry| entry.name == "a.tar.gz")
        .expect("archive listed");
    app.left_panel.select(Some(index));
    app
}

fn prompt(app: &App) -> &ExtractPrompt {
    let Overlay::ExtractPrompt(prompt) = &app.overlay else {
        panic!("the extract dialog is open");
    };
    prompt
}

/// Opening sets the variant; Esc takes it down and runs nothing else.
#[test]
fn open_sets_the_variant_and_esc_closes_it() {
    let mut app = App::new().expect("test setup");
    crate::ops::archive::open_extract(&mut app);
    assert!(matches!(app.overlay, Overlay::ExtractPrompt(_)));

    process_popup_input(&mut app, key(KeyCode::F(5)));
    assert!(
        matches!(app.overlay, Overlay::ExtractPrompt(_)),
        "other keys are consumed"
    );

    process_popup_input(&mut app, key(KeyCode::Esc));
    assert!(!app.overlay.is_active());
}

#[test]
fn typing_and_backspace_edit_the_destination() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::ExtractPrompt(Box::new(ExtractPrompt::new(
        Vec::new(),
        Path::new("/tmp"),
    ))));
    process_popup_input(&mut app, key(KeyCode::Char('/')));
    process_popup_input(&mut app, key(KeyCode::Char('x')));
    assert_eq!(prompt(&app).destination, "/tmp/x");
    process_popup_input(&mut app, key(KeyCode::Backspace));
    assert_eq!(prompt(&app).destination, "/tmp/");
    // A control chord is a command, never a character.
    process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
    );
    assert_eq!(prompt(&app).destination, "/tmp/");
}

#[test]
fn tab_and_arrows_walk_the_controls() {
    let temp = TempDir::new().expect("temp");
    pack(temp.path(), "a.tar.gz");
    let mut app = app_with_archive(temp.path(), temp.path());
    crate::ops::archive::open_extract(&mut app);

    process_popup_input(&mut app, key(KeyCode::Tab));
    assert_eq!(prompt(&app).field, ExtractField::Extract);
    process_popup_input(&mut app, key(KeyCode::Right));
    assert_eq!(prompt(&app).field, ExtractField::Cancel);
    process_popup_input(&mut app, key(KeyCode::BackTab));
    assert_eq!(prompt(&app).field, ExtractField::Extract);
    process_popup_input(&mut app, key(KeyCode::Up));
    assert_eq!(prompt(&app).field, ExtractField::Destination);
    process_popup_input(&mut app, key(KeyCode::Down));
    assert_eq!(prompt(&app).field, ExtractField::Extract);
}

#[test]
fn enter_extracts_into_the_other_panel_and_closes() {
    let temp = TempDir::new().expect("temp");
    let (left, right) = (temp.path().join("l"), temp.path().join("r"));
    fs::create_dir_all(&left).expect("left");
    fs::create_dir_all(&right).expect("right");
    pack(&left, "a.tar.gz");
    let mut app = app_with_archive(&left, &right);
    crate::ops::archive::open_extract(&mut app);

    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(!app.overlay.is_active());
    assert_eq!(
        fs::read_to_string(right.join("payload/hello.txt")).expect("unpacked"),
        "hello"
    );
}

#[test]
fn enter_on_cancel_closes_without_extracting() {
    let temp = TempDir::new().expect("temp");
    let (left, right) = (temp.path().join("l"), temp.path().join("r"));
    fs::create_dir_all(&left).expect("left");
    fs::create_dir_all(&right).expect("right");
    pack(&left, "a.tar.gz");
    let mut app = app_with_archive(&left, &right);
    crate::ops::archive::open_extract(&mut app);

    process_popup_input(&mut app, key(KeyCode::Tab));
    process_popup_input(&mut app, key(KeyCode::Tab));
    assert!(prompt(&app).on_cancel());
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(!app.overlay.is_active());
    assert!(!right.join("payload").exists(), "Cancel extracts nothing");
}
