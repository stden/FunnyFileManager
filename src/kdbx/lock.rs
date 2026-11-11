//! Purpose: Re-locking the vault — on request, and by the event loop's tick
//! after [`IDLE_LOCK`] without activity.

use std::time::Instant;

use super::{KdbxSession, KdbxState, UnlockPrompt, IDLE_LOCK};

impl KdbxSession {
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
