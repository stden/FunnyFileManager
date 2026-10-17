//! Purpose: the list and the view read their keys from `App.keymap` — a key the file moves acts in both and its old key does nothing, while the form and the exit question keep the dialog protocol whatever the keymap holds.

use super::*;
use crate::keymap::tables::LANGUAGE_SERVERS;
use crate::keymap::{ActionName, Explicit};
use crate::test_support::chord;

/// The keys `ffm.yml` would give the dialog: an action and its chords.
type File<'a> = [(&'static str, &'a [&'a str])];

/// The dialog open under the keys of `file`, the other actions on their
/// defaults.
fn opened_with(machine: &LspMachine, file: &File<'_>) -> App {
    let explicit: Explicit = file
        .iter()
        .map(|(action, keys)| {
            let chords = keys.iter().map(|key| chord(key)).collect();
            (ActionName::new(action), chords)
        })
        .collect();
    let mut app = machine.opened();
    app.keymap.language_servers = LANGUAGE_SERVERS.layered(&explicit).bindings;
    app
}

fn press_capital(app: &mut App, letter: char) {
    press_with(app, KeyCode::Char(letter), KeyModifiers::SHIFT);
}

#[test]
fn a_moved_key_acts_in_the_list_and_in_the_view() {
    let machine = LspMachine::new();
    let file: &File<'_> = &[("view", &["Ctrl+L"]), ("cursor_down", &["J"])];
    let mut app = opened_with(&machine, file);
    press_capital(&mut app, 'J');
    assert_eq!(selected(&app), "css");
    press_with(&mut app, KeyCode::Char('l'), KeyModifiers::CONTROL);
    assert_eq!(mode(&app), &Mode::View(ScrollTop::TOP));
    editor_mut(&mut app).record_drawn(Drawn::View {
        lines: 40,
        page: 10,
    });
    press_capital(&mut app, 'J');
    assert_eq!(mode(&app), &Mode::View(ScrollTop::TOP.down(1, 30)));
    press_with(&mut app, KeyCode::Char('l'), KeyModifiers::CONTROL);
    assert_eq!(mode(&app), &Mode::Browse);
    assert_eq!(selected(&app), "css", "the selection is unchanged");
}

#[test]
fn a_key_the_file_took_from_an_action_does_nothing() {
    let machine = LspMachine::new();
    let file: &File<'_> = &[
        ("view", &["Ctrl+L"]),
        ("cursor_down", &["J"]),
        ("toggle_enabled", &[]),
    ];
    let mut app = opened_with(&machine, file);
    let draft = editor(&app).draft().clone();
    for code in [KeyCode::F(3), KeyCode::Down, KeyCode::Char(' ')] {
        press(&mut app, code);
        assert_eq!(mode(&app), &Mode::Browse, "{code:?}");
        assert_eq!(selected(&app), "bat", "{code:?}");
        assert_eq!(editor(&app).draft(), &draft, "{code:?}");
    }
}

/// The form is a dialog and the exit question is one: Esc cancels both
/// with `close` on another key, and that key types itself in a field.
#[test]
fn the_form_and_the_exit_question_keep_the_dialog_protocol() {
    let machine = LspMachine::new();
    let file: &File<'_> = &[("close", &["Q"]), ("edit", &["E"])];
    let mut app = opened_with(&machine, file);
    select(&mut app, "css");
    press_capital(&mut app, 'E');
    assert!(matches!(mode(&app), Mode::Form(_)));
    let command = field(&mut app, FormField::Command);
    press_capital(&mut app, 'Q');
    assert_eq!(field(&mut app, FormField::Command), format!("{command}Q"));
    press(&mut app, KeyCode::Esc);
    assert_eq!(mode(&app), &Mode::Browse, "Esc cancels the form");
    press(&mut app, KeyCode::Char(' '));
    press(&mut app, KeyCode::Esc);
    assert_eq!(mode(&app), &Mode::Browse, "Esc closes nothing now");
    press_capital(&mut app, 'Q');
    assert_eq!(mode(&app), &Mode::ConfirmExit(ExitChoice::Save));
    press(&mut app, KeyCode::Esc);
    assert_eq!(mode(&app), &Mode::Browse, "Esc is the question's Cancel");
    press_capital(&mut app, 'Q');
    press(&mut app, KeyCode::Char('n'));
    assert!(matches!(app.overlay, Overlay::None));
}

/// What normalization changed for the dialog: Shift is in the space it
/// types, and Super is part of no chord.
#[test]
fn shift_with_space_toggles_and_super_is_not_read() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    press_with(&mut app, KeyCode::Char(' '), KeyModifiers::SHIFT);
    assert_eq!(editor(&app).draft().0["bat"].enabled, Enablement::Disabled);
    press_with(&mut app, KeyCode::F(3), KeyModifiers::SUPER);
    assert_eq!(mode(&app), &Mode::View(ScrollTop::TOP));
}
