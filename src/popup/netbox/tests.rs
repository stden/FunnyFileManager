//! Unit tests: the page a `NetBox` panel's list move takes, and the refusal
//! of a key that would act on the file list hidden beneath it.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

use super::refuse_entry_key;
use crate::models::{ListMotion, RowMotion};
use crate::test_support::netbox::app_in;

fn with(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

fn bare(code: KeyCode) -> KeyEvent {
    with(code, KeyModifiers::NONE)
}

#[test]
fn a_list_move_takes_the_panel_page() {
    assert_eq!(ListMotion::Up.with_page(7), RowMotion::Previous);
    assert_eq!(ListMotion::Down.with_page(7), RowMotion::Next);
    assert_eq!(ListMotion::Home.with_page(7), RowMotion::First);
    assert_eq!(ListMotion::End.with_page(7), RowMotion::Last);
    assert_eq!(ListMotion::PageUp.with_page(7), RowMotion::PageUp(7));
    assert_eq!(ListMotion::PageDown.with_page(7), RowMotion::PageDown(7));
}

#[test]
fn a_key_for_the_hidden_entries_is_consumed_with_the_reason() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    for entry_key in [
        bare(KeyCode::F(7)),
        bare(KeyCode::Insert),
        with(KeyCode::F(6), KeyModifiers::SHIFT),
        with(KeyCode::Delete, KeyModifiers::ALT),
        with(KeyCode::Char('a'), KeyModifiers::CONTROL),
        with(KeyCode::Char('x'), KeyModifiers::ALT),
    ] {
        app.set_status(crate::status::Status::Cleared);
        assert!(
            refuse_entry_key(&mut app, entry_key, "why"),
            "{entry_key:?}"
        );
        assert_eq!(app.status_text(), "why");
    }
}

#[test]
fn a_program_key_is_handed_back_untouched() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    app.set_status(crate::status::Status::CommandOutput("before".to_owned()));
    for program_key in [
        bare(KeyCode::Tab),
        bare(KeyCode::F(10)),
        bare(KeyCode::F(11)),
        bare(KeyCode::Char(':')),
        with(KeyCode::F(1), KeyModifiers::ALT),
        with(KeyCode::Char('o'), KeyModifiers::CONTROL),
    ] {
        assert!(
            !refuse_entry_key(&mut app, program_key, "why"),
            "{program_key:?}"
        );
    }
    assert_eq!(app.status_text(), "before");
}
