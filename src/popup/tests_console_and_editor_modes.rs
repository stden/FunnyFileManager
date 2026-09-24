//! The editor's unsaved-changes dialog.
//!
//! Console mode moved out with `ViewMode`: it is no longer an overlay, so its
//! keys are handled by the main-view dispatcher and its tests live in
//! `ui::tests::console_keys`.

use super::tests::{app_with_editor, key};
use super::{process_editor_input, process_popup_input};

use crate::models::{App, Overlay};

use crossterm::event::{KeyCode, KeyModifiers};

use tempfile::TempDir;

/// The editor key handler is normally reached only through the Editor overlay,
/// but it re-checks: with any other overlay open it must decline the key so the
/// caller can keep dispatching, and must not disturb that overlay.
#[test]
fn editor_keys_are_declined_when_no_editor_is_open() {
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::Help);

    assert!(!process_editor_input(
        &mut app,
        KeyCode::Esc,
        KeyModifiers::empty()
    ));
    assert!(
        matches!(app.overlay, Overlay::Help),
        "the help overlay must be left alone"
    );
}

/// The unsaved-changes dialog understands only Y / N / Esc. Any other key must
/// leave it standing — answering it by accident would drop or write the edits.
#[test]
fn unsaved_changes_dialog_ignores_unrelated_keys() {
    let temp = TempDir::new().expect("temp dir");
    let path = temp.path().join("file.txt");
    std::fs::write(&path, "ab\n").expect("write");
    let mut app = app_with_editor("ab\n", path.clone());

    process_popup_input(&mut app, key(KeyCode::Char('x')));
    process_popup_input(&mut app, key(KeyCode::Esc));
    assert!(
        app.get_editor().expect("editor").state.is_confirming_exit(),
        "setup: the dialog is up"
    );

    assert!(process_popup_input(&mut app, key(KeyCode::F(5))));

    let editor = app.get_editor().expect("the editor stays open");
    assert!(
        editor.state.is_confirming_exit(),
        "an unrelated key must not answer the dialog"
    );
    assert_eq!(editor.lines[0], "xab", "the edit is still in the buffer");
    assert_eq!(
        std::fs::read_to_string(&path).expect("read"),
        "ab\n",
        "nothing was written to disk"
    );
}
