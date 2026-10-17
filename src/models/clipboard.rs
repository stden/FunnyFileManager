//! Session-owned clipboard backend and optional secret-clear deadline.

use std::time::Instant;

use crate::clipboard::ClipboardBackend;

/// The session clipboard, plus an optional deadline for clearing a secret.
///
/// The backend is held for the whole session on purpose: on X11/Wayland the
/// clipboard contents survive only as long as the connection that set them, so
/// a per-copy `arboard::Clipboard` would put the password on the clipboard and
/// take it straight back off again when it dropped.
#[derive(Default)]
pub struct ClipboardSlot {
    backend: Option<Box<dyn ClipboardBackend>>,
    /// Clearing deadline for a copied secret, consumed by vault maintenance.
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
    /// A non-secret copy replaces any pending secret-clear deadline.
    pub fn disarm(&mut self) {
        self.clear_at = None;
    }

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
    #[cfg(feature = "keepass")]
    pub fn arm(&mut self, now: Instant) {
        self.clear_at = Some(now + crate::CLIPBOARD_CLEAR);
    }

    /// Whether the countdown has expired — and if so, disarm it.
    ///
    /// Consuming the deadline makes a caller's cleanup a one-shot operation.
    /// This state does not schedule or execute clipboard clearing itself.
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
