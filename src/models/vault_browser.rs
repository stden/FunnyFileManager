//! Vault-session state whose clone returns a locked session instead of
//! duplicating secrets. Shared clipboard ownership lives in `models::clipboard`.

use crate::kdbx::KdbxSession;

/// A `.kdbx` browsing session, wrapped so it can sit in the `Clone`-deriving
/// [`Overlay`](super::Overlay).
///
/// **Never add `#[derive(Debug)]`**: [`KdbxSession`] has none, and that absence
/// is what stops a `{:?}` anywhere in the app — `log_action` / `log_error`
/// included — from printing a master password.
pub struct KdbxOverlay(pub KdbxSession);

impl Clone for KdbxOverlay {
    /// A cloned session comes back **locked**, pointing at the same file.
    ///
    /// Deliberately not a secret-duplicating operation: copying the master key
    /// would double the number of buffers that must be zeroized and put one of
    /// them out of reach of `Esc` and the idle auto-lock, which only ever see
    /// the original. "The same vault, not yet unlocked" is the honest meaning
    /// of cloning a vault.
    fn clone(&self) -> Self {
        let mut session = KdbxSession::new(self.0.path.clone());
        session.access = self.0.access;
        Self(session)
    }
}

#[cfg(test)]
mod tests;
