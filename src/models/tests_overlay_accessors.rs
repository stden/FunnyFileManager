//! Unit tests: `App`'s overlay accessors and the value types the `:` command
//! line is built from.

use super::*;

fn new_app() -> App {
    App::new().expect("test setup: current dir is readable")
}

/// A one-row dropdown, for the cases that only need *some* menu.
fn menu_of_one() -> Menu {
    Menu::new(vec![Candidate {
        text: "cd".to_owned(),
        kind: CandidateKind::Word,
        hint: None,
    }])
    .expect("non-empty")
}

#[test]
fn test_get_overlay_hands_back_the_live_overlay_with_its_payload() {
    let mut app = new_app();
    assert!(matches!(app.get_overlay(), Overlay::None));

    app.set_overlay(Overlay::DeleteConfirm(vec![PathBuf::from("/tmp/doomed")]));
    let Overlay::DeleteConfirm(files) = app.get_overlay() else {
        panic!("expected the DeleteConfirm overlay back");
    };
    assert_eq!(files, &[PathBuf::from("/tmp/doomed")]);
}

#[test]
fn test_get_new_file_input_returns_the_name_typed_so_far() {
    let mut app = new_app();
    app.set_overlay(Overlay::NewFileInput("notes.txt".to_owned()));
    assert_eq!(app.get_new_file_input(), Some("notes.txt"));
}

#[test]
fn test_get_new_file_input_is_none_for_every_other_overlay() {
    let mut app = new_app();
    assert_eq!(app.get_new_file_input(), None, "no overlay open");

    app.set_overlay(Overlay::CommandLine("mkdir docs".into()));
    assert_eq!(
        app.get_new_file_input(),
        None,
        "a command line is not a new-file prompt"
    );
}

#[test]
fn test_get_delete_confirm_returns_every_pending_path() {
    let mut app = new_app();
    let doomed = vec![PathBuf::from("/tmp/a"), PathBuf::from("/tmp/b")];
    app.set_overlay(Overlay::DeleteConfirm(doomed.clone()));

    assert_eq!(app.get_delete_confirm(), Some(&doomed));
}

#[test]
fn test_get_delete_confirm_is_none_for_every_other_overlay() {
    let mut app = new_app();
    assert_eq!(app.get_delete_confirm(), None, "no overlay open");

    app.set_overlay(Overlay::NewFileInput("notes.txt".to_owned()));
    assert_eq!(
        app.get_delete_confirm(),
        None,
        "a new-file prompt has nothing to delete"
    );
}

#[test]
fn test_get_command_line_is_none_for_every_other_overlay() {
    let mut app = new_app();
    assert_eq!(app.get_command_line(), None, "no overlay open");

    app.set_overlay(Overlay::Help);
    assert_eq!(app.get_command_line(), None, "help is not the command line");
}

#[test]
fn test_take_delete_confirm_leaves_a_different_overlay_untouched() {
    // The happy path runs `std::mem::take` on the overlay; without the guard in
    // front of it, asking for delete targets would silently close whatever
    // dialog the user actually has open.
    let mut app = new_app();
    app.set_overlay(Overlay::CommandLine("cd /tmp".into()));

    assert_eq!(app.take_delete_confirm(), None);
    assert_eq!(app.get_command_line(), Some("cd /tmp"));
    assert!(app.is_overlay_active());
}

/// Taking the pending delete list consumes the overlay: F8-confirm must not
/// leave a stale dialog open after the paths have been handed off.
#[test]
fn test_take_delete_confirm_returns_the_paths_and_clears_the_overlay() {
    let mut app = new_app();
    let doomed = vec![PathBuf::from("/tmp/a"), PathBuf::from("/tmp/b")];
    app.set_overlay(Overlay::DeleteConfirm(doomed.clone()));

    assert_eq!(app.take_delete_confirm(), Some(doomed));
    assert!(
        !app.is_overlay_active(),
        "taking the list must close the confirm dialog"
    );
    assert_eq!(
        app.take_delete_confirm(),
        None,
        "a second take finds nothing left"
    );
}

#[test]
fn test_command_line_from_owned_string_keeps_the_text_and_closes_the_menu() {
    let line = CommandLine::from(String::from("cd /tmp"));

    assert_eq!(line.text(), "cd /tmp");
    assert_eq!(line.menu, None, "a fresh command line has no dropdown open");
}

#[test]
fn take_delete_confirm_is_none_for_other_overlays() {
    let mut app = new_app();
    assert!(app.take_delete_confirm().is_none());
    app.set_overlay(Overlay::Help);
    assert!(app.take_delete_confirm().is_none());
    assert!(
        matches!(app.overlay, Overlay::Help),
        "a miss must not disturb the overlay"
    );
}

/// An empty dropdown is unrepresentable, so `next`/`previous` have no
/// empty-list branch to get wrong and `selection` needs no `Option`.
#[test]
fn a_menu_cannot_be_built_from_an_empty_candidate_list() {
    assert!(Menu::new(Vec::new()).is_none());

    let mut menu = menu_of_one();
    assert_eq!(menu.selected(), 0);
    menu.next();
    assert_eq!(menu.selected(), 0, "a single row wraps onto itself");
    menu.previous();
    assert_eq!(menu.selected(), 0);
    assert_eq!(menu.selection().text, "cd");
}
