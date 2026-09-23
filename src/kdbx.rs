//! Purpose: the state machine behind the `.kdbx` (`KeePass`) browsing overlay.
//!
//! Unlock prompt, unlocked vault, incremental filter, reveal, idle auto-lock,
//! the `F4` entry edit form, confirmed deletion, and one-level persisted undo.
//!
//! # Why a state machine and not a bag of `Option`s
//!
//! [`KdbxState`] has exactly two shapes, so "entries without a master key" and
//! "a revealed password while locked" are not representable. Unlocking is the
//! only way to get a [`Vault`], and a [`Vault`] cannot exist without a
//! [`MasterKey`].
//!
//! # Secrets
//!
//! [`MasterKey`] and [`Revealed`] have no `Debug` and no `Display`, which makes
//! [`Vault`] and [`KdbxSession`] underivable in turn — a `{:?}` in
//! `log_action` / `log_error` cannot print a password even by accident. Errors
//! surfaced from here are fixed English strings (see [`unlock_error_message`]),
//! never the typed password. Secrets must not travel through the `Viewer` or
//! `Editor` overlays either: both keep scroll buffers alive for the rest of the
//! session.
//!
//! # Notes for the key-handling layer
//!
//! This module intentionally knows nothing about `crossterm`. The overlay's key
//! handler is expected to map keys onto [`KdbxSession::type_char`],
//! [`KdbxSession::backspace`], [`KdbxSession::move_selection`],
//! [`KdbxSession::reveal`], [`KdbxSession::copy_password`],
//! [`KdbxSession::copy_username`] and [`KdbxSession::lock`] — and to **swallow
//! every other key by returning `true`**. A `_ => false` fall-through in the
//! command-line overlay once let `F8` reach the panel and delete files; a
//! password list is a worse place for that bug.

mod browse;
pub(crate) mod cache;
pub mod edit;
mod persist;
pub mod secret;
mod structure;
mod unlock;
pub mod vault;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::keepass::KeepassError;

pub use edit::{EditField, EntryForm, FormMode};
pub use secret::{MasterKey, Revealed};
pub use unlock::UnlockPrompt;
pub use vault::{Motion, Vault};

/// Idle time after which an unlocked vault re-locks itself.
///
/// Enforced by [`KdbxSession::tick`] from the event loop — never a background
/// thread, which would race the renderer.
pub const IDLE_LOCK: Duration = Duration::from_mins(5);

/// Failures a caller of the unlocked API can hit.
///
/// Distinct from [`KeepassError`] so "the vault is locked" is not misreported as
/// "entry not found". No variant carries the master password.
#[derive(Debug)]
pub enum KdbxError {
    /// The vault is locked, so there is nothing to read.
    Locked,
    /// Nothing is under the cursor — usually a filter that matches no entry.
    NoSelection,
    NoEdit,
    NothingToUndo,
    ReadOnly,
    /// The underlying `.kdbx` read failed.
    Keepass(KeepassError),
}

impl std::fmt::Display for KdbxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Locked => write!(f, "Vault is locked"),
            Self::NoSelection => write!(f, "No entry selected"),
            Self::NoEdit => write!(f, "No entry is being edited"),
            Self::NothingToUndo => write!(f, "No KDBX change to undo"),
            Self::ReadOnly => write!(f, "View mode: open the database with F4 to edit entries"),
            Self::Keepass(error) => write!(f, "Cannot read entry: {error}"),
        }
    }
}

impl std::error::Error for KdbxError {}

/// A `.kdbx` file being browsed, and how far the user has got with it.
///
/// **No `Debug`** — see the module docs.
pub struct KdbxSession {
    pub path: PathBuf,
    pub state: KdbxState,
    pub(crate) access: Access,
    undo: UndoState,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum UndoState {
    Unavailable,
    Available,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Access {
    View,
    Edit,
}

/// Locked or unlocked; there is no third possibility.
pub enum KdbxState {
    Locked(UnlockPrompt),
    Unlocked(Vault),
}

impl KdbxState {
    pub fn is_locked(&self) -> bool {
        matches!(self, Self::Locked(_))
    }
}

impl KdbxSession {
    /// Start browsing `path`; the vault begins locked with an empty prompt.
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            state: KdbxState::Locked(UnlockPrompt::default()),
            access: Access::Edit,
            undo: UndoState::Unavailable,
        }
    }

    pub fn prompt(&self) -> Option<&UnlockPrompt> {
        match &self.state {
            KdbxState::Locked(prompt) => Some(prompt),
            KdbxState::Unlocked(_) => None,
        }
    }

    pub fn prompt_mut(&mut self) -> Option<&mut UnlockPrompt> {
        match &mut self.state {
            KdbxState::Locked(prompt) => Some(prompt),
            KdbxState::Unlocked(_) => None,
        }
    }

    pub fn vault(&self) -> Option<&Vault> {
        match &self.state {
            KdbxState::Unlocked(vault) => Some(vault),
            KdbxState::Locked(_) => None,
        }
    }

    pub fn vault_mut(&mut self) -> Option<&mut Vault> {
        match &mut self.state {
            KdbxState::Unlocked(vault) => Some(vault),
            KdbxState::Locked(_) => None,
        }
    }

    /// Re-lock: overwrite every secret and return to an empty prompt.
    ///
    /// The key file is dropped along with the password. It is not itself a
    /// secret, but re-picking it is the cheap half of re-authenticating and
    /// keeping it would make the locked state a partial credential cache.
    pub fn lock(&mut self) {
        if let KdbxState::Unlocked(vault) = &mut self.state {
            // Explicit, so the guarantee does not rest on `Drop` ordering
            // between the vault's fields.
            //
            // Deliberately kept even though no test can observe it: once the
            // state is replaced the vault is dropped and its buffers freed, so
            // reading them back would be use-after-free. `Vault::zeroize` is
            // covered directly instead (`tests_secrets.rs`). Do not delete this
            // call on the grounds that "nothing fails without it".
            vault.zeroize();
        }
        self.state = KdbxState::Locked(UnlockPrompt::default());
    }

    /// Lock the vault if it has been idle for [`IDLE_LOCK`]; returns whether
    /// this call is what locked it (so the caller can show a status message
    /// once, not on every tick).
    ///
    /// Call it from the event loop's tick. A background thread would be worse
    /// than useless here: it would mutate the session while the UI is drawing.
    pub fn tick(&mut self, now: Instant) -> bool {
        let Some(vault) = self.vault() else {
            return false;
        };
        // `duration_since` saturates at zero, so a clock that appears to go
        // backwards postpones the lock rather than triggering it.
        if now.duration_since(vault.last_activity()) < IDLE_LOCK {
            return false;
        }
        self.lock();
        true
    }

    /// Record user activity without changing anything else.
    ///
    /// Needed for the actions that cannot take `now` themselves — see
    /// [`Self::copy_username`].
    pub fn touch(&mut self, now: Instant) {
        if let Some(vault) = self.vault_mut() {
            vault.touch(now);
        }
    }
}

/// Turn an unlock failure into a message that is safe to display.
///
/// Mapped per variant rather than forwarded verbatim: `KeepassError::Open`
/// wraps a third-party crate's text, and this string is shown in the UI and may
/// be near logging code, so nothing derived from the credentials is allowed
/// through. The wrong-password case is also merged with the corrupt-file case
/// on purpose — the two are indistinguishable to a caller anyway.
fn unlock_error_message(error: &KeepassError) -> String {
    match error {
        KeepassError::Io(io) if io.kind() == std::io::ErrorKind::NotFound => {
            "File not found".to_owned()
        }
        KeepassError::Io(io) => format!("Cannot read file: {io}"),
        KeepassError::Open(_) => "Wrong password or key file".to_owned(),
        // Save / EntryNotFound / AlreadyExists / Serialize cannot arise from a
        // read, but their Display text is fixed and secret-free.
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests_edit;
#[cfg(test)]
mod tests_filter;
#[cfg(test)]
mod tests_nav;
#[cfg(test)]
mod tests_secrets;
// `pub(crate)`: the popup and ui overlay tests unlock the same fixture vaults
// instead of restating the cheap-KDF setup a third time.
#[cfg(test)]
pub(crate) mod tests_support;
#[cfg(test)]
mod tests_unlock;
