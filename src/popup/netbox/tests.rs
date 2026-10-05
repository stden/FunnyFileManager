//! Unit tests: the list keys a `NetBox` panel moves by, and the refusal of a
//! key that would act on the file list hidden beneath it.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

use super::{is_bare, refuse_entry_key, row_motion};
use crate::models::RowMotion;
use crate::test_support::netbox::app_in;

fn with(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

fn bare(code: KeyCode) -> KeyEvent {
    with(code, KeyModifiers::NONE)
}

#[test]
fn the_list_keys_are_the_bare_or_shifted_arrows_and_pages() {
    let table = [
        (KeyCode::Up, RowMotion::Previous),
        (KeyCode::Down, RowMotion::Next),
        (KeyCode::Home, RowMotion::First),
        (KeyCode::End, RowMotion::Last),
        (KeyCode::PageUp, RowMotion::PageUp(7)),
        (KeyCode::PageDown, RowMotion::PageDown(7)),
    ];
    for (code, motion) in table {
        assert_eq!(row_motion(bare(code), 7), Some(motion), "{code:?}");
        assert_eq!(row_motion(with(code, KeyModifiers::SHIFT), 7), Some(motion));
        assert_eq!(row_motion(with(code, KeyModifiers::CONTROL), 7), None);
        assert_eq!(row_motion(with(code, KeyModifiers::ALT), 7), None);
    }
    assert_eq!(row_motion(bare(KeyCode::Enter), 7), None);
    assert_eq!(row_motion(bare(KeyCode::Char('j')), 7), None);
}

#[test]
fn a_bare_key_has_no_modifier_at_all() {
    assert!(is_bare(bare(KeyCode::Enter), KeyCode::Enter));
    assert!(!is_bare(bare(KeyCode::Enter), KeyCode::F(3)));
    for modifier in [
        KeyModifiers::SHIFT,
        KeyModifiers::CONTROL,
        KeyModifiers::ALT,
    ] {
        assert!(!is_bare(with(KeyCode::F(3), modifier), KeyCode::F(3)));
    }
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

#[test]
fn a_key_with_any_other_modifier_is_not_bare_either() {
    for modifier in [KeyModifiers::SUPER, KeyModifiers::META, KeyModifiers::HYPER] {
        assert!(!is_bare(with(KeyCode::Enter, modifier), KeyCode::Enter));
    }
}
