//! Purpose: The dialogs with a focus to move — the kill confirmation, the
//! priority form and the confirmation `[` asks for — each keeping what it
//! froze and a focus that is always on one of its own buttons.

use crate::lineedit::LineEditor;

use super::super::renice::NicePlan;
use super::super::targets::FrozenTargets;
use super::super::{KillOffer, NiceStep, Niceness, Signal};

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

/// Where the priority form's focus is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FormFocus {
    #[default]
    Field,
    Set,
    Cancel,
}

impl FormFocus {
    const fn next(self) -> Self {
        match self {
            Self::Field => Self::Set,
            Self::Set => Self::Cancel,
            Self::Cancel => Self::Field,
        }
    }

    const fn previous(self) -> Self {
        match self {
            Self::Field => Self::Cancel,
            Self::Set => Self::Field,
            Self::Cancel => Self::Set,
        }
    }
}

/// The `F7` form: a nice value for the frozen targets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriorityForm {
    targets: FrozenTargets,
    value: LineEditor,
    focus: FormFocus,
}

impl PriorityForm {
    /// The form for `targets`, offering their current nice value.
    #[must_use]
    pub fn new(targets: FrozenTargets, current: Niceness) -> Self {
        Self {
            targets,
            value: LineEditor::new(&current.get().to_string()),
            focus: FormFocus::Field,
        }
    }

    #[must_use]
    pub const fn targets(&self) -> &FrozenTargets {
        &self.targets
    }

    #[must_use]
    pub const fn value(&self) -> &LineEditor {
        &self.value
    }

    pub const fn value_mut(&mut self) -> &mut LineEditor {
        &mut self.value
    }

    /// The nice value typed, or `None` while the text is not one: Set is
    /// disabled then.
    #[must_use]
    pub fn niceness(&self) -> Option<Niceness> {
        let number = self.value.text().trim().parse::<i32>().ok()?;
        Niceness::try_from(number).ok()
    }

    /// Up and Down: one step, stopping at -20 and 19; text that is not a
    /// nice value is left for the user to fix.
    pub fn step(&mut self, step: NiceStep) {
        let Some(stepped) = self.niceness().and_then(|nice| nice.step(step)) else {
            return;
        };
        self.value.set_text(&stepped.get().to_string());
    }

    #[must_use]
    pub const fn focus(&self) -> FormFocus {
        self.focus
    }

    /// Tab: the next of field, Set and Cancel.
    pub const fn focus_next(&mut self) {
        self.focus = self.focus.next();
    }

    /// Shift+Tab: the previous one.
    pub const fn focus_previous(&mut self) {
        self.focus = self.focus.previous();
    }

    /// Focuses `focus`: a click names its button before pressing it.
    pub const fn focus_on(&mut self, focus: FormFocus) {
        self.focus = focus;
    }
}

/// A button of the confirmation `[` asks for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ReniceButton {
    /// Lowers the priority, as planned.
    #[default]
    Renice,
    Cancel,
}

/// `[` for a user who is not root: lowering a priority is confirmed first,
/// because only root can raise it again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NiceConfirm {
    plan: NicePlan,
    focus: ReniceButton,
}

impl NiceConfirm {
    /// The confirmation of `plan`, its Renice button focused.
    #[must_use]
    pub fn new(plan: NicePlan) -> Self {
        Self {
            plan,
            focus: ReniceButton::default(),
        }
    }

    #[must_use]
    pub const fn plan(&self) -> &NicePlan {
        &self.plan
    }

    /// The plan, to apply once the confirmation closes.
    #[must_use]
    pub fn into_plan(self) -> NicePlan {
        self.plan
    }

    #[must_use]
    pub const fn focused(&self) -> ReniceButton {
        self.focus
    }

    /// Focuses `button`: a click names its button before pressing it.
    pub const fn focus_on(&mut self, button: ReniceButton) {
        self.focus = button;
    }

    /// Tab, Left and Right: the other button.
    pub const fn focus_other(&mut self) {
        self.focus = match self.focus {
            ReniceButton::Renice => ReniceButton::Cancel,
            ReniceButton::Cancel => ReniceButton::Renice,
        };
    }
}
