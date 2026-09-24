//! Purpose: closing the Language servers dialog — at once when clean, through the Save / Discard / Cancel question when changed.

use super::*;

#[test]
fn esc_closes_a_clean_dialog_at_once() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    press(&mut app, KeyCode::Esc);
    assert!(matches!(app.overlay, Overlay::None));
    let mut app = machine.opened();
    press(&mut app, KeyCode::F(10));
    assert!(matches!(app.overlay, Overlay::None));
}

/// Space toggles; Esc on a changed dialog asks, and the exit question has the
/// editor's keys.
#[test]
fn a_changed_dialog_asks_before_closing() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    press(&mut app, KeyCode::Char(' '));
    assert_eq!(editor(&app).draft().0["bat"].enabled, Enablement::Disabled);
    press(&mut app, KeyCode::Esc);
    assert_eq!(mode(&app), &Mode::ConfirmExit(ExitChoice::Save));
    press(&mut app, KeyCode::Right);
    assert_eq!(mode(&app), &Mode::ConfirmExit(ExitChoice::Discard));
    press(&mut app, KeyCode::Tab);
    assert_eq!(mode(&app), &Mode::ConfirmExit(ExitChoice::Cancel));
    press(&mut app, KeyCode::Left);
    assert_eq!(mode(&app), &Mode::ConfirmExit(ExitChoice::Discard));
    press(&mut app, KeyCode::Char('c'));
    assert_eq!(mode(&app), &Mode::Browse, "Cancel returns to the list");
    press(&mut app, KeyCode::F(10));
    press(&mut app, KeyCode::Esc);
    assert_eq!(mode(&app), &Mode::Browse, "Esc is Cancel too");
}

#[test]
fn no_discards_the_changes_and_writes_nothing() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    press(&mut app, KeyCode::Char(' '));
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Char('n'));
    assert!(matches!(app.overlay, Overlay::None));
    assert!(matches!(
        app.status,
        crate::status::Status::ChangesDiscarded
    ));
    assert!(!machine.settings().exists());
}

#[test]
fn yes_saves_and_closes_and_f2_saves_in_place() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    press(&mut app, KeyCode::Char(' '));
    press(&mut app, KeyCode::F(2));
    assert!(machine.settings().exists());
    assert!(!editor(&app).dirty());
    press(&mut app, KeyCode::Char(' '));
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.overlay, Overlay::None), "Enter on Yes saves");
    assert_eq!(app.config.lsp.0["bat"].enabled, Enablement::Enabled);
}

/// A save the draft cannot make returns to the list with the reason.
#[test]
fn a_refused_save_from_the_exit_question_returns_to_the_list() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    select(&mut app, "python");
    press(&mut app, KeyCode::F(4));
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    assert_eq!(focus(&mut app), FormField::Extensions);
    press(&mut app, KeyCode::End);
    type_text(&mut app, " ts");
    press(&mut app, KeyCode::Enter);
    assert_eq!(
        mode(&app),
        &Mode::Browse,
        "Enter on a passive list presses OK"
    );
    press(&mut app, KeyCode::Esc);
    assert_eq!(mode(&app), &Mode::ConfirmExit(ExitChoice::Save));
    press(&mut app, KeyCode::Char('y'));
    assert_eq!(mode(&app), &Mode::Browse);
    assert!(matches!(
        editor(&app).notice(),
        Some(Notice::Error(message)) if message.starts_with("Not saved: .ts is claimed by")
    ));
    assert!(!machine.settings().exists());
}
