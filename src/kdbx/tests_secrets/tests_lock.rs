//! Purpose: the idle auto-lock and explicit locking — the exact deadline, the
//! activity that postpones it, and what a lock discards.

use std::time::Duration;

use crate::kdbx::tests_support::{focus, unlocked_sample, unlocked_sample_with_keyfile};
use crate::kdbx::{KdbxError, IDLE_LOCK};

#[test]
fn tick_locks_once_the_idle_limit_is_reached_and_not_a_moment_before() {
    let (_dir, mut session, t0) = unlocked_sample();

    assert!(!session.tick(t0));
    assert!(!session.tick(t0 + IDLE_LOCK.saturating_sub(Duration::from_millis(1))));
    assert!(
        session.vault().is_some(),
        "still unlocked one millisecond short of the limit"
    );

    assert!(session.tick(t0 + IDLE_LOCK), "locks exactly at the limit");
    assert!(session.vault().is_none());
    assert!(session.state.is_locked());

    // Already locked — no second "just locked" report on later ticks.
    assert!(!session.tick(t0 + IDLE_LOCK * 2));
}

#[test]
fn user_activity_postpones_the_idle_lock() {
    let (_dir, mut session, t0) = unlocked_sample();
    let late = t0 + IDLE_LOCK.saturating_sub(Duration::from_secs(1));

    // A real action, not just a bare `touch`: typing into the filter counts.
    session.type_char('g', late);

    assert!(
        !session.tick(t0 + IDLE_LOCK),
        "the clock restarted at `late`"
    );
    assert!(session.tick(late + IDLE_LOCK));
}

#[test]
fn revealing_and_copying_also_count_as_activity() {
    let (_dir, mut session, t0) = unlocked_sample();
    focus(&mut session, "GitHub", t0);
    let late = t0 + IDLE_LOCK.saturating_sub(Duration::from_secs(1));

    assert!(session.reveal(late).expect("reveal"));
    assert!(!session.tick(t0 + IDLE_LOCK));

    let later = late + Duration::from_secs(1);
    assert_eq!(session.copy_password(later).expect("copy"), "gh-pw");
    assert!(!session.tick(later + IDLE_LOCK.saturating_sub(Duration::from_millis(1))));
    assert!(session.tick(later + IDLE_LOCK));
}

#[test]
fn locking_discards_the_credentials_and_leaves_an_empty_prompt() {
    // A key-file vault, so "the prompt comes back empty" says something about
    // the key file instead of being vacuously true.
    let (_dir, _key_dir, mut session, now) = unlocked_sample_with_keyfile();
    assert!(session.reveal(now).expect("reveal"));

    session.lock();

    let prompt = session.prompt().expect("locked again");
    assert!(prompt.is_empty());
    assert_eq!(prompt.masked(), "");
    assert!(prompt.error().is_none());
    assert!(
        prompt.keyfile().is_none(),
        "the locked state must not keep half the credentials"
    );
    // The master key is gone: a read now fails as `Locked` instead of
    // succeeding from a leftover credential.
    assert!(matches!(session.copy_password(now), Err(KdbxError::Locked)));
}
