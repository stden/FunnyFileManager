//! Purpose: F7 with a path makes the whole chain of folders, as Far does.

use super::*;
use crate::status::Status;

fn make_folder(app: &mut App, name: &str) {
    let mut prompt = NamePrompt::make_folder();
    name.chars().for_each(|ch| prompt.push(ch));
    app.set_overlay(Overlay::NamePrompt(prompt));
    press(app, KeyCode::Enter);
}

/// `p1/p2/p3` used to close the dialog having made nothing and said nothing.
#[test]
fn a_nested_name_makes_every_folder_on_the_way() {
    let (mut app, dir) = app_over(&[]);

    make_folder(&mut app, "p1/p2/p3");

    let made = dir.path().join("p1/p2/p3");
    assert!(made.is_dir(), "{}", app.status);
    assert_eq!(app.status, Status::CreatedDir(made.display().to_string()));
    let cursor = app.get_active_panel_ref().selected_path();
    assert_eq!(cursor, Some(dir.path().join("p1")), "the cursor is on it");
}

/// Part of the chain may be there already; all of it is refused.
#[test]
fn a_nested_name_that_exists_already_is_refused() {
    let (mut app, dir) = app_over(&[]);
    fs::create_dir_all(dir.path().join("q1/q2")).expect("test setup");

    make_folder(&mut app, "q1/q3");
    assert!(dir.path().join("q1/q3").is_dir(), "{}", app.status);

    make_folder(&mut app, "q1/q2");
    assert_eq!(app.status, Status::AlreadyExists("q1/q2".into()));
}
