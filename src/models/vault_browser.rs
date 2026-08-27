//! Purpose: the two pieces of `.kdbx` application state that [`App`](super::App)
//! owns — the vault-browser session and the clipboard a copied secret goes to.
//!
//! Both live here rather than in [`crate::kdbx`] / [`crate::clipboard`] because
//! both have to satisfy something those modules deliberately refuse to provide:
//! `App` derives `Clone`, while a master key and an OS clipboard handle are
//! exactly the two things that must not be duplicated. The wrappers make the
//! `Clone` derive legal *and* say, at the type level, what a clone gets instead.

use std::time::Instant;

use crate::clipboard::ClipboardBackend;
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
        Self(KdbxSession::new(self.0.path.clone()))
    }
}

/// The clipboard a copied secret goes to, plus the deadline it is wiped at.
///
/// The backend is held for the whole session on purpose: on X11/Wayland the
/// clipboard contents survive only as long as the connection that set them, so
/// a per-copy `arboard::Clipboard` would put the password on the clipboard and
/// take it straight back off again when it dropped.
#[derive(Default)]
pub struct ClipboardSlot {
    backend: Option<Box<dyn ClipboardBackend>>,
    /// When the copied secret must be wiped, or `None` when nothing of ours is
    /// on the clipboard. Armed by a copy, disarmed by the event loop's tick.
    clear_at: Option<Instant>,
}

impl Clone for ClipboardSlot {
    /// A clone gets **no** backend and **no** pending wipe.
    ///
    /// There is one OS clipboard and one owner of it. Handing a second `App`
    /// the right to wipe it would let the loser of the race erase whatever the
    /// winner copied next.
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl ClipboardSlot {
    /// Install the backend copies go through — once, from the production entry
    /// point. Tests install a recording backend instead of touching a real one.
    pub fn install(&mut self, backend: Box<dyn ClipboardBackend>) {
        self.backend = Some(backend);
    }

    /// `+ 'static` matches what the `Box` in the slot actually holds; leaving
    /// the bound elided would tie it to `&mut self`, which `&mut`'s invariance
    /// then refuses to reconcile with the boxed object's `'static`.
    pub fn backend_mut(&mut self) -> Option<&mut (dyn ClipboardBackend + 'static)> {
        self.backend.as_deref_mut()
    }

    /// Start the auto-clear countdown for a secret just put on the clipboard.
    ///
    /// Re-arming *replaces* the previous deadline instead of queueing a second
    /// one: an older timer left running would fire part-way through the newer
    /// secret's countdown and wipe a password the user had not pasted yet.
    pub fn arm(&mut self, now: Instant) {
        self.clear_at = Some(now + crate::CLIPBOARD_CLEAR);
    }

    /// Whether the countdown has expired — and if so, disarm it.
    ///
    /// Disarming here is what keeps the wipe a one-shot: the tick runs on every
    /// loop iteration, and a deadline left in place would re-clear the
    /// clipboard on every keystroke from then on, including over a value the
    /// user copied from another application in the meantime.
    pub fn take_if_due(&mut self, now: Instant) -> bool {
        if self.clear_at.is_none_or(|at| now < at) {
            return false;
        }
        self.clear_at = None;
        true
    }

    /// Whether a copied secret is still waiting to be wiped.
    pub const fn is_armed(&self) -> bool {
        self.clear_at.is_some()
    }
}

#[cfg(test)]
mod tests;
