//! Unit tests: the PATH editor's list reads its keys from the keymap — a
//! modifier more is another key, another keymap moves the commands — while
//! the entry field and the two questions keep the dialog protocol.

use super::tests::{add, dialog, fixture, key};
use super::*;
use crate::keymap::{tables::PATH_EDITOR, ActionName, Explicit};
use crate::test_support::chord;

/// Presses the key written `text`, as the loop hands it to the dialog.
fn press(app: &mut App, text: &str) {
    let event = chord(text).to_event();
    assert!(process(app, event.code, event.modifiers), "{text}");
}

/// The keymap of a file that names `entries`, each action with its keys.
fn remap(app: &mut App, entries: &[(&'static str, &[&str])]) {
    let file: Explicit = entries
        .iter()
        .map(|&(action, keys)| {
            let keys = keys.iter().map(|text| chord(text)).collect();
            (ActionName::new(action), keys)
        })
        .collect();
    app.keymap.path_editor = PATH_EDITOR.layered(&file).bindings;
}

#[test]
fn a_modifier_more_is_another_key_and_the_list_swallows_it() {
    let (temp, mut app) = fixture();
    add(&mut app, &temp.path().to_string_lossy());
    add(&mut app, &temp.path().join("second").to_string_lossy());
    let second = dialog(&app).entries[1].text.clone();
    for text in [
        "Shift+Up",
        "Alt+Home",
        "Ctrl+Shift+Up",
        "Shift+Del",
        "Alt+F8",
        "Shift+F2",
        "Shift+Ins",
        "Alt+Enter",
        "Shift+F4",
        "Shift+Esc",
    ] {
        press(&mut app, text);
        assert!(matches!(dialog(&app).mode, Mode::Browse), "{text}");
        assert_eq!(dialog(&app).selected, 1, "{text}");
        assert_eq!(dialog(&app).entries.len(), 2, "{text}");
        assert_eq!(dialog(&app).entries[1].text, second, "{text}");
    }
}

#[test]
fn the_list_obeys_the_keys_of_another_keymap() {
    let (temp, mut app) = fixture();
    add(&mut app, &temp.path().to_string_lossy());
    add(&mut app, &temp.path().join("second").to_string_lossy());
    remap(
        &mut app,
        &[("save", &["Ctrl+S"]), ("cursor_up", &["K"]), ("add", &[])],
    );

    press(&mut app, "F2");
    assert!(
        matches!(dialog(&app).mode, Mode::Browse),
        "F2 saves no more"
    );
    press(&mut app, "Up");
    assert_eq!(dialog(&app).selected, 1, "Up moves no more");
    press(&mut app, "Ins");
    press(&mut app, "F7");
    assert!(matches!(dialog(&app).mode, Mode::Browse), "no key adds");

    press(&mut app, "K");
    assert_eq!(dialog(&app).selected, 0);
    press(&mut app, "Ctrl+S");
    assert!(matches!(dialog(&app).mode, Mode::ConfirmSave));
}

#[test]
fn the_entry_field_and_the_questions_keep_their_keys_under_another_keymap() {
    let (temp, mut app) = fixture();
    remap(&mut app, &[("close", &["F10"]), ("edit", &["F4"])]);

    add(&mut app, &temp.path().to_string_lossy());
    assert_eq!(dialog(&app).entries.len(), 1, "Enter accepts the entry");
    key(&mut app, KeyCode::Enter);
    assert!(
        matches!(dialog(&app).mode, Mode::Browse),
        "Enter edits no more"
    );
    key(&mut app, KeyCode::F(4));
    assert!(matches!(dialog(&app).mode, Mode::Edit { .. }));
    key(&mut app, KeyCode::Esc);
    assert!(
        matches!(dialog(&app).mode, Mode::Browse),
        "Esc leaves the field"
    );

    key(&mut app, KeyCode::Esc);
    assert!(
        matches!(dialog(&app).mode, Mode::Browse),
        "Esc closes no more"
    );
    key(&mut app, KeyCode::F(10));
    assert!(matches!(dialog(&app).mode, Mode::ConfirmDiscard));
    key(&mut app, KeyCode::Esc);
    assert!(matches!(dialog(&app).mode, Mode::Browse), "Esc answers no");
    key(&mut app, KeyCode::F(10));
    key(&mut app, KeyCode::Enter);
    assert!(matches!(app.overlay, Overlay::None), "Enter answers yes");
}

#[test]
fn an_unchanged_list_closes_at_once_on_the_key_of_the_keymap() {
    let (_temp, mut app) = fixture();
    remap(&mut app, &[("close", &["F10"])]);
    key(&mut app, KeyCode::Esc);
    assert!(matches!(app.overlay, Overlay::PathEditor(_)));
    key(&mut app, KeyCode::F(10));
    assert!(matches!(app.overlay, Overlay::None));
}
