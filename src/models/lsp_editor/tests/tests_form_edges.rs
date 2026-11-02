//! Purpose: Accepting a server form when none is open changes nothing.

use super::*;

#[test]
fn accepting_while_no_form_is_open_does_nothing() {
    let mut editor = editor(&json!({}));
    assert_eq!(editor.accept_form(), None);
    assert_eq!(editor.mode(), &Mode::Browse);
}

#[test]
fn a_server_with_a_control_character_cannot_open_its_form() {
    let mut editor = editor(&json!({}));
    editor.draft.0.get_mut("python").expect("python").command = "py\u{7}".to_owned();
    let opened = editor.open_form(&FormTarget::Existing(language("python")));
    assert!(matches!(opened, Err(Notice::Error(_))));
}

#[test]
fn a_server_missing_from_the_list_cannot_open_its_form() {
    let mut editor = editor(&json!({}));
    let opened = editor.open_form(&FormTarget::Existing(language("no-such-server")));
    assert!(matches!(opened, Err(Notice::Error(_))));
}
