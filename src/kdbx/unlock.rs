//! Purpose: the locked half of a session — the masked unlock prompt and the unlock attempt.

use std::path::{Path, PathBuf};
use std::time::Instant;

use super::{unlock_error_message, KdbxSession, KdbxState, MasterKey, Vault};
use crate::keepass;

/// The unlock dialog: a masked password, an optional key file, and the reason
/// the last attempt failed.
pub struct UnlockPrompt {
    /// Wrapped in [`MasterKey`] from the first keystroke so a half-typed
    /// password is zeroized too — including when the user gives up and hits
    /// `Esc`.
    password: MasterKey,
    keyfile: Option<PathBuf>,
    error: Option<String>,
    reveal: bool,
}

impl Default for UnlockPrompt {
    fn default() -> Self {
        Self {
            password: MasterKey::typing(),
            keyfile: None,
            error: None,
            reveal: false,
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

    pub fn visible_text(&self) -> String {
        if self.reveal {
            self.password.expose().to_owned()
        } else {
            self.masked()
        }
    }

    pub fn toggle_reveal(&mut self) {
        self.reveal = !self.reveal;
    }

    pub fn is_revealed(&self) -> bool {
        self.reveal
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

        match keepass::read_catalog(&self.path, typed.expose(), keyfile.as_deref()) {
            Ok(catalog) => {
                let vault = Vault::new(typed, keyfile, catalog.entries, catalog.groups, now);
                self.state = KdbxState::Unlocked(vault);
                true
            }
            Err(error) => {
                prompt.error = Some(unlock_error_message(&error));
                false
            }
        }
    }
}
