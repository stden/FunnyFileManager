//! Rendering tests for the vault-browser overlay.
//!
//! The module's contract is about what may reach the screen, so every test
//! here asserts on the drawn buffer itself: bullets instead of a typed master
//! password, [`MASK`] instead of entry passwords, and a revealed secret only
//! when `Ctrl+R` arranged it.

use crate::ui::Cells;
use std::path::PathBuf;
use std::time::Instant;

use ratatui::{backend::TestBackend, Terminal};

use super::{render_kdbx, Columns, COLUMN_GAP, MARKER_WIDTH, MASK, PASSWORD_WIDTH};
use crate::kdbx::tests_support::{
    focus, password_for, type_password, unlocked_sample, vault_file, SAMPLE, VAULT_PASSWORD,
};
use crate::kdbx::KdbxSession;

/// Every cell of one drawn frame, rows joined with newlines.
fn draw(session: &KdbxSession) -> String {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| render_kdbx(f, session)).expect("draw");
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
fn locked_prompt_shows_bullets_never_the_typed_characters() {
    let mut session = KdbxSession::new(PathBuf::from("vault.kdbx"));
    type_password(&mut session, "abc", Instant::now());

    let screen = draw(&session);

    assert!(screen.contains("KeePass: vault.kdbx"));
    assert!(screen.contains("Master password:"));
    assert!(
        screen.contains("•••_"),
        "one bullet per typed character plus the cursor"
    );
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
fn unlocked_list_masks_every_password() {
    let (_dir, session, _now) = unlocked_sample();

    let screen = draw(&session);

    assert!(screen.contains("5 of 5 entries"), "{screen}");
    assert!(screen.contains(MASK));
    for fixture in SAMPLE {
        assert!(
            !screen.contains(fixture.password),
            "{} leaked its password to the screen",
            fixture.title
        );
    }
}

#[test]
fn reveal_draws_the_password_on_the_cursor_row_and_the_filter_narrows_the_count() {
    let (_dir, mut session, now) = unlocked_sample();
    focus(&mut session, "GitHub", now);
    session.reveal(now).expect("reveal");

    let screen = draw(&session);

    assert!(screen.contains(password_for("GitHub")), "{screen}");
    assert!(screen.contains("1 of 5 entries"), "{screen}");
}

#[test]
fn columns_never_exceed_the_width_they_were_fitted_to() {
    // Below this floor the fixed columns alone overflow and the body is
    // clamped to its 3-cell minimum; everything wider must fit exactly.
    let gaps = COLUMN_GAP + COLUMN_GAP + COLUMN_GAP;
    let floor = MARKER_WIDTH + PASSWORD_WIDTH + gaps + Cells::new(3);
    for width in [0, 10, floor.get(), 60, 80, 200].map(Cells::new) {
        let columns = Columns::fit(width);
        let used =
            MARKER_WIDTH + columns.title + columns.username + columns.group + PASSWORD_WIDTH + gaps;
        assert!(used <= width.max(floor), "fit({width}) used {used}");
    }
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
fn the_full_ui_dispatches_the_kdbx_overlay() {
    let mut app = crate::models::App::new().expect("test app");
    app.set_overlay(crate::models::Overlay::Kdbx(crate::models::KdbxOverlay(
        KdbxSession::new(PathBuf::from("vault.kdbx")),
    )));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| crate::ui::ui(f, &mut app)).expect("draw");
    let buf = terminal.backend().buffer();
    let screen: String = (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(screen.contains("KeePass: vault.kdbx"), "{screen}");
}
