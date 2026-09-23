//! Security-relevant tests: reveal/copy behaviour, zeroization, the idle
//! auto-lock, and the structural guarantee that no secret-carrying type can be
//! formatted at all.

use std::fmt;
use std::marker::PhantomData;

use super::secret::{MasterKey, Revealed};
use super::tests_support::unlocked_sample;
use super::{EntryForm, KdbxError, KdbxSession, KdbxState, Motion, UnlockPrompt, Vault};

#[cfg(test)]
mod tests_lock;
#[cfg(test)]
mod tests_reveal;

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
    let mut key = MasterKey::typing();
    key.pop();
    assert!(key.is_empty());
    assert_eq!(key.char_count(), 0);
}
