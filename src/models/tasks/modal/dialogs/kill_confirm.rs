//! Purpose: The red confirmation every signal goes through: the targets it
//! froze, the buttons its offer has, and which of them is focused.

use super::super::super::targets::FrozenTargets;
use super::super::super::{KillOffer, Signal};

/// A button of the kill confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmButton {
    Terminate,
    Kill,
    /// Sends the signal chosen in the `F9` menu.
    Send,
    Cancel,
}

/// F8 and Del's buttons: Terminate focused, then Kill where signals exist.
#[cfg(unix)]
const TERMINATE_OR_KILL: &[ConfirmButton] = &[
    ConfirmButton::Terminate,
    ConfirmButton::Kill,
    ConfirmButton::Cancel,
];
/// Without signals there is only ending a process.
#[cfg(not(unix))]
const TERMINATE_OR_KILL: &[ConfirmButton] = &[ConfirmButton::Terminate, ConfirmButton::Cancel];
const SEND: &[ConfirmButton] = &[ConfirmButton::Send, ConfirmButton::Cancel];

/// The red confirmation every signal goes through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KillConfirm {
    targets: FrozenTargets,
    offer: KillOffer,
    /// Always less than the offer's button count.
    focus: usize,
}

impl KillConfirm {
    /// The confirmation for `targets`, its first button focused.
    #[must_use]
    pub const fn new(targets: FrozenTargets, offer: KillOffer) -> Self {
        Self {
            targets,
            offer,
            focus: 0,
        }
    }

    #[must_use]
    pub const fn targets(&self) -> &FrozenTargets {
        &self.targets
    }

    #[must_use]
    pub const fn offer(&self) -> KillOffer {
        self.offer
    }

    /// The buttons, left to right.
    #[must_use]
    pub const fn buttons(&self) -> &'static [ConfirmButton] {
        match self.offer {
            KillOffer::TerminateOrKill => TERMINATE_OR_KILL,
            KillOffer::Send(_) => SEND,
        }
    }

    #[must_use]
    pub fn focused(&self) -> ConfirmButton {
        let buttons = self.buttons();
        let focused = buttons.get(self.focus).or_else(|| buttons.last());
        focused.copied().unwrap_or(ConfirmButton::Cancel)
    }

    /// Tab and Right: the next button, wrapping.
    pub fn focus_next(&mut self) {
        self.focus = (self.focus + 1) % self.buttons().len();
    }

    /// Left: the previous button, wrapping.
    pub fn focus_previous(&mut self) {
        let count = self.buttons().len();
        self.focus = (self.focus + count - 1) % count;
    }

    /// Focuses `button` if this confirmation has it.
    pub fn focus_on(&mut self, button: ConfirmButton) {
        if let Some(at) = self.buttons().iter().position(|each| *each == button) {
            self.focus = at;
        }
    }

    /// The signal `button` sends, or `None` for Cancel and for a button
    /// this confirmation does not have.
    #[must_use]
    pub fn signal(&self, button: ConfirmButton) -> Option<Signal> {
        if !self.buttons().contains(&button) {
            return None;
        }
        match (button, self.offer) {
            (ConfirmButton::Terminate, _) => Some(Signal::Terminate),
            (ConfirmButton::Kill, _) => Some(Signal::Kill),
            (ConfirmButton::Send, KillOffer::Send(signal)) => Some(signal),
            (ConfirmButton::Send, KillOffer::TerminateOrKill) | (ConfirmButton::Cancel, _) => None,
        }
    }
}
