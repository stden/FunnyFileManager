//! Purpose: The `F7` priority form: a nice value typed or stepped for the
//! targets it froze, and a focus on the field, Set or Cancel.

use crate::lineedit::LineEditor;

use super::super::super::targets::FrozenTargets;
use super::super::super::{NiceStep, Niceness};

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
