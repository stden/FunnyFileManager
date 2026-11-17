//! Purpose: The confirmation `[` asks a user who is not root before lowering
//! a priority only root can raise again, and its focused button.

use super::super::super::renice::NicePlan;

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
