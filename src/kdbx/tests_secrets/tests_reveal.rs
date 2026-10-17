//! Purpose: reveal and copy behaviour — on-demand fetch, copying without a
//! reveal side effect, and the cursor choosing the row.

use std::time::Instant;

use crate::kdbx::tests_support::{
    focus, password_for, selected_title, type_password, unlocked_sample, username_for, vault_file,
    Fixture, VAULT_PASSWORD,
};
use crate::kdbx::{KdbxSession, Motion};
use tempfile::TempDir;

#[test]
fn reveal_fetches_on_demand_and_toggles_back_off() {
    let (_dir, mut session, now) = unlocked_sample();
    focus(&mut session, "GitHub", now);
    assert!(!session.vault().expect("unlocked").is_revealed());

    assert!(session.reveal(now).expect("reveal on"));
    assert_eq!(session.vault().expect("unlocked").revealed(), Some("gh-pw"));

    assert!(!session.reveal(now).expect("reveal off"));
    assert!(session.vault().expect("unlocked").revealed().is_none());
    assert!(!session.vault().expect("unlocked").is_revealed());

    // And it can be fetched again afterwards.
    assert!(session.reveal(now).expect("reveal on again"));
    assert_eq!(session.vault().expect("unlocked").revealed(), Some("gh-pw"));
}

#[test]
fn copying_a_password_does_not_require_revealing_it_first() {
    let (_dir, mut session, now) = unlocked_sample();
    focus(&mut session, "GitHub", now);

    assert_eq!(session.copy_password(now).expect("copy"), "gh-pw");

    // Copying must not put the secret on screen as a side effect.
    assert!(!session.vault().expect("unlocked").is_revealed());
    assert!(session.vault().expect("unlocked").revealed().is_none());
}

#[test]
fn copying_follows_the_cursor() {
    let (_dir, mut session, now) = unlocked_sample();
    let first = selected_title(&session).expect("a selection");
    assert_eq!(session.copy_username().expect("copy"), username_for(&first));

    session.move_selection(Motion::Next, now);

    let second = selected_title(&session).expect("a selection");
    assert_ne!(first, second, "the cursor actually moved");
    assert_eq!(
        session.copy_username().expect("copy"),
        username_for(&second)
    );
    assert_eq!(
        session.copy_password(now).expect("copy"),
        password_for(&second)
    );
}

/// A root-group vault holding two entries titled `GitHub`, unlocked; each
/// entry's password is `pw-` followed by its user name.
fn unlocked_twins() -> (TempDir, KdbxSession, Instant) {
    let twins = [
        Fixture {
            group: &[],
            title: "GitHub",
            username: "alice",
            url: "",
            password: "pw-alice",
        },
        Fixture {
            group: &[],
            title: "GitHub",
            username: "bob",
            url: "",
            password: "pw-bob",
        },
    ];
    let (dir, path) = vault_file(&twins, VAULT_PASSWORD, None);
    let now = Instant::now();
    let mut session = KdbxSession::new(path);
    type_password(&mut session, VAULT_PASSWORD, now);
    assert!(session.unlock(now), "fixture vault must unlock");
    (dir, session, now)
}

/// Two entries may share a title. The row under the cursor is the one whose
/// password is copied — resolving by name would always give the first.
#[test]
fn a_duplicate_title_copies_the_password_of_the_selected_row() {
    let (_dir, mut session, now) = unlocked_twins();

    // Whatever order the two were listed in, each row copies its own secret.
    for _ in 0..2 {
        let user = session
            .vault()
            .expect("unlocked")
            .selected_entry()
            .expect("a row is selected")
            .username
            .clone();
        assert_eq!(
            session.copy_password(now).expect("copy"),
            format!("pw-{user}"),
            "the {user} row must copy the {user} password"
        );
        session.move_selection(Motion::Next, now);
    }
}
