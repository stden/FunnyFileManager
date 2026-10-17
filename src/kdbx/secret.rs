//! Purpose: the two in-memory secrets an unlocked `.kdbx` vault holds, wrapped
//! in newtypes that overwrite their bytes instead of just dropping them.
//!
//! `crate::keepass` hands back plaintext in ordinary `String`s, and a `String`
//! is freed without being cleared — the plaintext then sits in the freed
//! allocation until an unrelated allocation happens to reuse it. Everything in
//! this module exists to shorten that window.

/// Overwrite every byte of `bytes` with zero.
///
/// The writes go through `write_volatile` so the optimizer may not elide them as
/// "stores nobody reads afterwards" — which is exactly what it would conclude
/// about a buffer that is about to be freed.
#[allow(
    unsafe_code,
    reason = "only a volatile write keeps the optimizer from eliding the zeroing"
)]
fn zeroize_bytes(bytes: &mut [u8]) {
    for byte in bytes {
        // SAFETY: `byte` is a live, initialized, in-bounds `&mut u8`.
        unsafe { std::ptr::write_volatile(byte, 0) };
    }
}

/// Overwrite `secret`'s bytes **in place**, then empty it.
///
/// `String::clear` alone only moves the length to 0; the plaintext stays in the
/// allocation.
#[allow(
    unsafe_code,
    reason = "`as_mut_vec` is the only way to overwrite a `String`'s bytes in place"
)]
pub(crate) fn zeroize_string(secret: &mut String) {
    // SAFETY: every byte is overwritten with 0, which is valid UTF-8, so the
    // `String` is still well-formed when the borrow ends.
    unsafe {
        let bytes = secret.as_mut_vec();
        let cap = bytes.capacity();
        bytes.resize(cap, 0);
        zeroize_bytes(bytes);
    };
    secret.clear();
}

/// Bytes reserved up front for a password being typed.
///
/// A `String::push` that outgrows the capacity reallocates, and the *old* buffer
/// is released with the partial password still in it — out of reach of
/// [`zeroize_string`], which can only clear the buffer the `String` currently
/// owns. 64 bytes covers any realistic master password, so in practice the
/// growth path is never taken.
const TYPED_CAPACITY: usize = 64;

/// The master password of an unlocked vault.
///
/// It has to be *kept*: `crate::keepass::read_entry` re-opens and re-derives the
/// database on every single read, so there is no session handle to hold instead.
///
/// **Never add `#[derive(Debug)]`, `#[derive(Clone)]`, or a `Display` impl.**
/// The absence of `Debug` is load-bearing rather than cosmetic: it is what makes
/// [`Vault`](super::Vault) and [`KdbxSession`](super::KdbxSession) underivable
/// too, so no `{:?}` anywhere in the app — including `log_action` / `log_error`,
/// which leaked a secret exactly once already — can print the master password.
/// `src/kdbx/tests_secrets.rs` asserts this structurally.
pub struct MasterKey(String);

impl MasterKey {
    /// Take ownership of an already-assembled secret.
    pub fn new(secret: String) -> Self {
        Self(secret)
    }

    /// An empty key with room to grow, for a password being typed character by
    /// character (see [`TYPED_CAPACITY`]).
    pub fn typing() -> Self {
        Self(String::with_capacity(TYPED_CAPACITY))
    }

    /// Append one typed character.
    pub fn push(&mut self, ch: char) {
        let needed = self.0.len() + ch.len_utf8();
        if needed > self.0.capacity() {
            let mut next = String::with_capacity(self.0.capacity().saturating_mul(2).max(needed));
            next.push_str(&self.0);
            zeroize_string(&mut self.0);
            self.0 = next;
        }
        self.0.push(ch);
    }

    /// Remove the last typed character, zeroizing the bytes it occupied.
    #[allow(
        unsafe_code,
        reason = "`as_mut_vec` is the only way to overwrite a `String`'s bytes in place"
    )]
    pub fn pop(&mut self) {
        let Some(last) = self.0.chars().next_back() else {
            return;
        };
        let keep = self.0.len() - last.len_utf8();
        // SAFETY: `keep` is the start of the last character and therefore a char
        // boundary, so truncating there leaves valid UTF-8. Zeroing first is the
        // point: plain `String::pop` only shortens the length and leaves the
        // removed character readable in the buffer's spare capacity.
        unsafe {
            let bytes = self.0.as_mut_vec();
            #[expect(
                clippy::indexing_slicing,
                reason = "`keep` is `len` minus the last character's width, so `keep..` is in bounds; a broken invariant must stop loudly, never skip the wipe"
            )]
            zeroize_bytes(&mut bytes[keep..]);
            bytes.truncate(keep);
        }
    }

    /// Number of characters typed so far — the only thing the renderer may know.
    pub fn char_count(&self) -> usize {
        self.0.chars().count()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Hand the plaintext to `crate::keepass`. Crate-private on purpose: the UI
    /// layer has no reason to ever see it.
    pub(crate) fn expose(&self) -> &str {
        &self.0
    }

    /// Overwrite the secret now rather than waiting for [`Drop`].
    pub fn zeroize(&mut self) {
        zeroize_string(&mut self.0);
    }
}

impl Drop for MasterKey {
    fn drop(&mut self) {
        zeroize_string(&mut self.0);
    }
}

/// One entry password, fetched only by an explicit `Ctrl+R` reveal.
///
/// Same rule as [`MasterKey`]: **no `Debug`, no `Display`, no `Clone`.**
pub struct Revealed(String);

impl Revealed {
    pub fn new(secret: String) -> Self {
        Self(secret)
    }

    /// The plaintext, for the one widget that is meant to draw it.
    pub(crate) fn expose(&self) -> &str {
        &self.0
    }

    /// Overwrite the secret now rather than waiting for [`Drop`].
    pub fn zeroize(&mut self) {
        zeroize_string(&mut self.0);
    }
}

impl Drop for Revealed {
    fn drop(&mut self) {
        zeroize_string(&mut self.0);
    }
}
