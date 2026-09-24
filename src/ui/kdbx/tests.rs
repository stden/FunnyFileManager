//! Rendering tests for the vault-browser overlay while it is locked; the
//! unlocked browser's are in `tests_unlocked.rs`.
//!
//! The module's contract is about what may reach the screen, so every test
//! here asserts on the drawn buffer itself: bullets instead of a typed master
//! password, [`MASK`] instead of entry passwords, and a revealed secret only
//! when `Ctrl+R` arranged it.

use std::path::PathBuf;
use std::time::Instant;

use ratatui::{backend::TestBackend, Terminal};

use super::render_kdbx;
use crate::kdbx::tests_support::{type_password, vault_file, SAMPLE, VAULT_PASSWORD};
use crate::kdbx::KdbxSession;

/// Every cell of one drawn frame, rows joined with newlines.
pub(super) fn draw(session: &KdbxSession) -> String {
    draw_at(session, 80, 24)
}

pub(super) fn draw_at(session: &KdbxSession, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|f| render_kdbx(f, session, &mut crate::mouse::DialogMouse::default()))
        .expect("draw");
    let buf = terminal.backend().buffer();
    (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn master_password_button_reveals_the_typed_value() {
    let mut session = KdbxSession::new(PathBuf::from("C:/vaults/private/vault.kdbx"));
    type_password(&mut session, "master-secret", Instant::now());
    session.prompt_mut().unwrap().toggle_reveal();

    let screen = draw(&session);

    assert!(screen.contains("master-secret"), "{screen}");
    assert!(screen.contains("◉"), "{screen}");
}

#[test]
fn locked_prompt_shows_bullets_never_the_typed_characters() {
    let mut session = KdbxSession::new(PathBuf::from("C:/vaults/private/vault.kdbx"));
    type_password(&mut session, "abc", Instant::now());

    let screen = draw(&session);

    assert!(screen.contains("C:/vaults/private/vault.kdbx"), "{screen}");
    assert!(screen.contains("Master password:"));
    assert!(screen.contains("•••"), "one bullet per typed character");
    assert!(
        !screen.contains("abc"),
        "the typed password must never be drawn"
    );
}

#[test]
fn failed_unlock_draws_the_fixed_error_beside_the_field() {
    let (_dir, path) = vault_file(SAMPLE, VAULT_PASSWORD, None);
    let now = Instant::now();
    let mut session = KdbxSession::new(path);
    type_password(&mut session, "not the password", now);
    assert!(!session.unlock(now));
    let error = session
        .prompt()
        .expect("still locked")
        .error()
        .expect("reason recorded")
        .to_owned();

    let screen = draw(&session);

    assert!(screen.contains(&error), "{screen}");
    assert!(!screen.contains("not the password"));
}

#[test]
fn locked_prompt_shows_the_chosen_key_file() {
    let mut session = KdbxSession::new(PathBuf::from("vault.kdbx"));
    session
        .prompt_mut()
        .expect("locked")
        .set_keyfile(Some(PathBuf::from("/keys/vault.key")));

    let screen = draw(&session);

    assert!(screen.contains("Key file: /keys/vault.key"), "{screen}");
}

#[test]
fn locked_form_uses_far_fields_and_positions_the_cursor() {
    let mut session = KdbxSession::new(PathBuf::from("vault.kdbx"));
    type_password(&mut session, "abc", Instant::now());
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("terminal");
    terminal
        .draw(|frame| render_kdbx(frame, &session, &mut crate::mouse::DialogMouse::default()))
        .expect("draw");
    let buffer = terminal.backend().buffer().clone();
    assert!(buffer
        .content()
        .iter()
        .any(|cell| cell.bg == crate::ui::palette::DIALOG_BG));
    let bullet = buffer
        .content()
        .iter()
        .find(|cell| cell.symbol() == "•")
        .expect("mask");
    assert_eq!(bullet.bg, crate::ui::palette::FIELD_BG);
    assert_eq!(bullet.fg, crate::ui::palette::FIELD_FG);
    let position = terminal.get_cursor_position().expect("cursor");
    assert_eq!(buffer[(position.x - 1, position.y)].symbol(), "•");
}

#[test]
fn long_master_password_keeps_cursor_inside_the_masked_field() {
    let mut session = KdbxSession::new(PathBuf::from("vault.kdbx"));
    type_password(&mut session, &"secret".repeat(40), Instant::now());
    for width in [20, 40, 80] {
        let mut terminal = Terminal::new(TestBackend::new(width, 24)).expect("terminal");
        let mut tracker = crate::mouse::DialogMouse::default();
        terminal
            .draw(|frame| render_kdbx(frame, &session, &mut tracker))
            .expect("draw");
        let cursor = terminal.get_cursor_position().expect("cursor");
        assert!(tracker.body.expect("body").contains(cursor));
        assert_eq!(
            terminal.backend().buffer()[(cursor.x, cursor.y)].bg,
            crate::ui::palette::FIELD_BG
        );
        assert_eq!(tracker.buttons.len(), 3);
    }
}
