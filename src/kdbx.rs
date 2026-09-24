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
mod error;
mod lock;
mod persist;
pub mod secret;
mod structure;
mod unlock;
pub mod vault;

use std::path::PathBuf;
use std::time::Duration;

pub use edit::{EditField, EntryForm, FormMode};
use error::unlock_error_message;
pub use error::KdbxError;
pub use secret::{MasterKey, Revealed};
pub use unlock::UnlockPrompt;
pub use vault::{Motion, Vault};

/// Idle time after which an unlocked vault re-locks itself.
///
/// Enforced by [`KdbxSession::tick`] from the event loop — never a background
/// thread, which would race the renderer.
pub const IDLE_LOCK: Duration = Duration::from_mins(5);

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
