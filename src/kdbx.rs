//! Purpose: the state machine behind the `.kdbx` (`KeePass`) browsing overlay.
//!
//! Unlock prompt, unlocked vault, incremental filter, reveal, and idle
//! auto-lock. Read-only: nothing here creates, edits, or deletes entries.
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

pub mod secret;
pub mod vault;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::keepass::{self, EntryDetails, KeepassError};

pub use secret::{MasterKey, Revealed};
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
    /// The underlying `.kdbx` read failed.
    Keepass(KeepassError),
}

impl std::fmt::Display for KdbxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Locked => write!(f, "Vault is locked"),
            Self::NoSelection => write!(f, "No entry selected"),
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

/// The unlock dialog: a masked password, an optional key file, and the reason
/// the last attempt failed.
pub struct UnlockPrompt {
    /// Wrapped in [`MasterKey`] from the first keystroke so a half-typed
    /// password is zeroized too — including when the user gives up and hits
    /// `Esc`.
    password: MasterKey,
    keyfile: Option<PathBuf>,
    error: Option<String>,
}

impl Default for UnlockPrompt {
    fn default() -> Self {
        Self {
            password: MasterKey::typing(),
            keyfile: None,
            error: None,
        }
    }
}

impl UnlockPrompt {
    pub fn push_char(&mut self, ch: char) {
        // A new keystroke means the user is retrying; stop showing the stale
        // failure next to fresh input.
        self.error = None;
        self.password.push(ch);
    }

    pub fn pop_char(&mut self) {
        self.error = None;
        self.password.pop();
    }

    /// What the renderer draws: one bullet per typed character, never the text.
    /// The plaintext has no public accessor at all, so it cannot be rendered,
    /// logged, or put in a status message by mistake.
    pub fn masked(&self) -> String {
        "•".repeat(self.password.char_count())
    }

    pub fn typed_len(&self) -> usize {
        self.password.char_count()
    }

    pub fn is_empty(&self) -> bool {
        self.password.is_empty()
    }

    pub fn set_keyfile(&mut self, keyfile: Option<PathBuf>) {
        self.keyfile = keyfile;
    }

    pub fn keyfile(&self) -> Option<&Path> {
        self.keyfile.as_deref()
    }

    /// Why the previous unlock attempt failed, ready to display.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

impl KdbxSession {
    /// Start browsing `path`; the vault begins locked with an empty prompt.
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            state: KdbxState::Locked(UnlockPrompt::default()),
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

    /// Try the typed credentials. Returns whether the vault is now unlocked.
    ///
    /// The typed password leaves the prompt either way: on success it moves into
    /// the [`Vault`], on failure it is dropped (and therefore zeroized), because
    /// a *wrong* secret is still a secret and has no business sitting in memory
    /// waiting for the next attempt.
    pub fn unlock(&mut self, now: Instant) -> bool {
        let KdbxState::Locked(prompt) = &mut self.state else {
            return true;
        };
        let typed = std::mem::replace(&mut prompt.password, MasterKey::typing());
        let keyfile = prompt.keyfile.clone();

        match keepass::list_entries(&self.path, typed.expose(), keyfile.as_deref()) {
            Ok(entries) => {
                self.state = KdbxState::Unlocked(Vault::new(typed, keyfile, entries, now));
                true
            }
            Err(error) => {
                prompt.error = Some(unlock_error_message(&error));
                false
            }
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

    /// Route one printable character: it edits the password while locked and the
    /// entry filter while unlocked.
    ///
    /// A single entry point on purpose — two separate ones would eventually let
    /// a master password be typed into the (rendered in clear) filter box.
    pub fn type_char(&mut self, ch: char, now: Instant) {
        match &mut self.state {
            KdbxState::Locked(prompt) => prompt.push_char(ch),
            KdbxState::Unlocked(vault) => vault.push_filter_char(ch, now),
        }
    }

    /// Backspace, routed the same way as [`Self::type_char`].
    pub fn backspace(&mut self, now: Instant) {
        match &mut self.state {
            KdbxState::Locked(prompt) => prompt.pop_char(),
            KdbxState::Unlocked(vault) => vault.pop_filter_char(now),
        }
    }

    /// Move the cursor over the filtered list; a no-op while locked.
    pub fn move_selection(&mut self, motion: Motion, now: Instant) {
        if let Some(vault) = self.vault_mut() {
            vault.move_selection(motion, now);
        }
    }

    /// Toggle `Ctrl+R` reveal for the selected entry. Returns whether the
    /// password is now on screen.
    ///
    /// Revealing fetches on demand: nothing keeps an entry password in memory
    /// until the user explicitly asks for it.
    pub fn reveal(&mut self, now: Instant) -> Result<bool, KdbxError> {
        if let Some(vault) = self.vault_mut() {
            vault.touch(now);
            if vault.is_revealed() {
                vault.hide();
                return Ok(false);
            }
        }
        let details = self.read_selected_entry()?;
        let vault = self.vault_mut().ok_or(KdbxError::Locked)?;
        vault.set_revealed(Revealed::new(details.password));
        Ok(true)
    }

    /// The selected entry's password, for the caller to put on the clipboard.
    ///
    /// Re-reads the database instead of requiring a prior [`Self::reveal`], so
    /// copying never forces the user to put the secret on screen first. It is
    /// returned, never printed or logged — the caller owns it and should drop it
    /// as soon as the clipboard has it.
    pub fn copy_password(&mut self, now: Instant) -> Result<String, KdbxError> {
        let details = self.read_selected_entry()?;
        self.touch(now);
        Ok(details.password)
    }

    /// The selected entry's user name.
    ///
    /// Takes `&self` because it reads only the secret-free summary list; the key
    /// handler must call [`Self::touch`] to register the activity.
    pub fn copy_username(&self) -> Result<String, KdbxError> {
        let vault = self.vault().ok_or(KdbxError::Locked)?;
        let entry = vault.selected_entry().ok_or(KdbxError::NoSelection)?;
        Ok(entry.username.clone())
    }

    /// Re-open the database and read the selected entry in full.
    ///
    /// Private, and deliberately so: the returned [`EntryDetails`] *does* derive
    /// `Debug` and holds a plaintext password, so it must not escape this module
    /// except as the single `String` the caller asked for.
    fn read_selected_entry(&self) -> Result<EntryDetails, KdbxError> {
        let vault = self.vault().ok_or(KdbxError::Locked)?;
        let entry = vault.selected_entry().ok_or(KdbxError::NoSelection)?;
        // `group_path` is kept structural rather than joined: `KeePass` allows
        // '/' inside a group name, so a joined string would resolve to the wrong
        // entry.
        let group: Vec<&str> = entry.group_path.iter().map(String::as_str).collect();
        keepass::read_entry(
            &self.path,
            vault.master().expose(),
            vault.keyfile(),
            &group,
            &entry.title,
        )
        .map_err(KdbxError::Keepass)
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
