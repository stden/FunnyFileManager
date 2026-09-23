//! Security-relevant tests: reveal/copy behaviour, zeroization, the idle
//! auto-lock, and the structural guarantee that no secret-carrying type can be
//! formatted at all.

use std::fmt;
use std::marker::PhantomData;
use std::time::{Duration, Instant};

use super::secret::{MasterKey, Revealed};
use super::tests_support::{
    focus, password_for, selected_title, type_password, unlocked_sample,
    unlocked_sample_with_keyfile, username_for, vault_file, Fixture, VAULT_PASSWORD,
};
use super::{EntryForm, KdbxError, KdbxSession, KdbxState, Motion, UnlockPrompt, Vault, IDLE_LOCK};
use tempfile::TempDir;

// ── "does `T` implement `Debug`?", answered at run time ──────────────────
//
// Inherent methods are preferred over trait methods, but an inherent method
// whose `where` clause is unsatisfied is discarded from the candidate set — so
// the call lands on the blanket trait impl exactly when `T` is *not* `Debug`.
//
// It has to be a macro, not a generic `fn`: inside a generic function the
// receiver type is still abstract, `T: Debug` is not known to hold, and the
// inherent candidate is discarded for *every* type — the probe would answer
// "no" unconditionally and prove nothing.

struct DebugProbe<T>(PhantomData<T>);

trait MaybeDebug {
    fn implements_debug(&self) -> bool {
        false
    }
}

impl<T> MaybeDebug for DebugProbe<T> {}

impl<T: fmt::Debug> DebugProbe<T> {
    // `self` exists only to make this a method, which is what gives it priority
    // over the trait's default body; there is nothing to read from it.
    #[allow(clippy::unused_self)]
    fn implements_debug(&self) -> bool {
        true
    }
}

macro_rules! implements_debug {
    ($type:ty) => {
        DebugProbe::<$type>(PhantomData).implements_debug()
    };
}

#[test]
fn no_type_holding_a_secret_can_be_formatted() {
    // Positive control: the probe genuinely detects a `Debug` impl. Without
    // this, a probe that always answered "no" would pass the assertions below
    // while proving nothing.
    assert!(implements_debug!(crate::keepass::EntrySummary));
    assert!(implements_debug!(KdbxError));
    assert!(implements_debug!(Motion));

    // The guarantee: `{:?}` cannot print a password anywhere — not in
    // `log_action`, not in `log_error`, not in a status message, not in a panic
    // payload — because none of these types is formattable in the first place.
    assert!(!implements_debug!(MasterKey));
    assert!(!implements_debug!(Revealed));
    assert!(!implements_debug!(UnlockPrompt));
    assert!(!implements_debug!(Vault));
    assert!(!implements_debug!(KdbxState));
    assert!(!implements_debug!(KdbxSession));
    // The edit draft holds the entry password in a `MasterKey`, so the form is
    // as unformattable as the vault that owns it.
    assert!(!implements_debug!(EntryForm));
}

#[test]
#[allow(unsafe_code)]
fn zeroize_overwrites_the_secret_in_place_rather_than_just_shortening_it() {
    let mut key = MasterKey::new("hunter2-master".to_owned());
    let ptr = key.expose().as_ptr();
    let len = key.expose().len();

    key.zeroize();

    assert!(key.expose().is_empty());
    // SAFETY: `key` is still alive and zeroizing only resets the length, so the
    // allocation is the same one, still at least `len` bytes long, and every
    // byte in it was written by `zeroize`.
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    assert!(
        bytes.iter().all(|&byte| byte == 0),
        "plaintext survived zeroize: {bytes:?}"
    );
}

#[test]
#[allow(unsafe_code)]
fn revealed_zeroize_overwrites_the_secret_in_place() {
    let mut revealed = Revealed::new("entry-password".to_owned());
    let ptr = revealed.expose().as_ptr();
    let len = revealed.expose().len();

    revealed.zeroize();

    assert!(revealed.expose().is_empty());
    // SAFETY: as above — same allocation, still owned by a live `Revealed`.
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    assert!(
        bytes.iter().all(|&byte| byte == 0),
        "plaintext survived zeroize: {bytes:?}"
    );
}

#[test]
#[allow(unsafe_code)]
fn deleting_a_typed_character_erases_it_from_the_buffer() {
    let mut key = MasterKey::typing();
    for ch in "abcd".chars() {
        key.push(ch);
    }
    let ptr = key.expose().as_ptr();

    key.pop();

    assert_eq!(key.char_count(), 3);
    // SAFETY: the buffer is reserved up front and all four bytes were written
    // by `push` before `pop` shortened the length; `key` still owns it.
    let bytes = unsafe { std::slice::from_raw_parts(ptr, 4) };
    assert_eq!(
        bytes, b"abc\0",
        "the deleted character stayed readable in spare capacity"
    );
}

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

#[test]
#[allow(unsafe_code)]
fn zeroizing_a_vault_wipes_the_master_key_and_the_revealed_password() {
    let (_dir, mut session, now) = unlocked_sample();
    assert!(session.reveal(now).expect("reveal"));

    let vault = session.vault_mut().expect("unlocked");
    let key_ptr = vault.master().expose().as_ptr();
    let key_len = vault.master().expose().len();
    assert!(key_len > 0, "the fixture password is not empty");

    vault.zeroize();

    assert!(vault.revealed().is_none());
    assert!(vault.master().expose().is_empty());
    // SAFETY: `vault` — and therefore the master key's allocation — is still
    // alive; `zeroize` only reset the length after overwriting the bytes.
    let bytes = unsafe { std::slice::from_raw_parts(key_ptr, key_len) };
    assert!(
        bytes.iter().all(|&byte| byte == 0),
        "the master password survived: {bytes:?}"
    );
}

#[test]
fn popping_an_empty_master_key_is_a_no_op() {
    let mut key = crate::kdbx::MasterKey::typing();
    key.pop();
    assert!(key.is_empty());
    assert_eq!(key.char_count(), 0);
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
