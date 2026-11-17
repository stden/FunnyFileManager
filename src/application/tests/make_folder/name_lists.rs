//! Purpose: F7's "Process multiple names" checkbox — the name field as a `;`-separated list, and where a list stops.

use super::*;

/// Far's "Process multiple names": the field becomes a list, and one dialog
/// makes every folder in it.
#[test]
fn the_checkbox_turns_the_name_field_into_a_list() {
    let (mut app, dir) = app_over(&[]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));
    type_name(&mut app, "one;two;three");
    // Down twice to the checkbox — the Target field is greyed — then space.
    press(
        &mut app,
        &[
            KeyCode::Down,
            KeyCode::Down,
            KeyCode::Char(' '),
            KeyCode::Enter,
        ],
    );

    for name in ["one", "two", "three"] {
        assert!(dir.path().join(name).is_dir(), "{name} should exist");
    }
    assert_eq!(cursor_on(&mut app), "one", "the first of the list");
}

/// Without the checkbox a semicolon is an ordinary character in a name, which
/// is what makes the checkbox worth having.
#[test]
fn a_semicolon_is_part_of_the_name_until_the_checkbox_is_ticked() {
    let (mut app, dir) = app_over(&[]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));
    type_name(&mut app, "one;two");
    press(&mut app, &[KeyCode::Enter]);

    assert!(dir.path().join("one;two").is_dir());
    assert!(!dir.path().join("one").exists());
}

/// The space bar types a space in the name field. It would be a poor trade to
/// gain a checkbox and lose the ability to name a folder "my notes".
#[test]
fn the_space_bar_is_still_a_space_in_the_name_field() {
    let (mut app, dir) = app_over(&[]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));
    type_name(&mut app, "my notes");
    press(&mut app, &[KeyCode::Enter]);
    assert!(dir.path().join("my notes").is_dir());
}

/// The first failure stops the run: carrying on would leave the user reading
/// about the last name while an earlier one silently did not appear.
#[test]
fn a_list_stops_at_the_first_name_it_cannot_make() {
    let (mut app, dir) = app_over(&["blocked"]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));
    type_name(&mut app, "made;blocked;never");
    press(
        &mut app,
        &[
            KeyCode::Down,
            KeyCode::Down,
            KeyCode::Char(' '),
            KeyCode::Enter,
        ],
    );

    assert!(dir.path().join("made").is_dir());
    assert!(!dir.path().join("never").exists(), "the run stopped");
    assert!(app.status.to_string().contains("already exists"));
}
