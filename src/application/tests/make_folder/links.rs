//! Purpose: F7's link drop-down — a symbolic link is made when it says so, and a greyed target field is skipped.

use super::*;

#[cfg(unix)]
#[test]
fn a_symbolic_link_is_made_when_the_drop_down_says_so() {
    let (mut app, dir) = app_over(&["target"]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));
    type_name(&mut app, "shortcut");
    // Down to the drop-down, Right to choose the link, Down to the target.
    press(&mut app, &[KeyCode::Down, KeyCode::Right, KeyCode::Down]);
    type_name(&mut app, "target");
    press(&mut app, &[KeyCode::Enter]);

    let link = dir.path().join("shortcut");
    assert_eq!(
        fs::read_link(&link).expect("a link"),
        std::path::Path::new("target"),
        "the link points where the Target field said"
    );
    assert_eq!(cursor_on(&mut app), "shortcut");
}

/// The Target field is greyed until a link is chosen, so Down from the
/// drop-down must skip it rather than parking the cursor on dead text.
#[test]
fn the_cursor_skips_the_target_field_while_it_is_greyed() {
    use crate::models::NameField;

    let (mut app, _dir) = app_over(&[]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));
    press(&mut app, &[KeyCode::Down, KeyCode::Down]);
    let Overlay::NamePrompt(prompt) = &app.overlay else {
        panic!("the dialog is open");
    };
    assert_eq!(prompt.link(), LinkType::None);
    assert_eq!(prompt.field(), NameField::Multiple, "Target was skipped");
}
