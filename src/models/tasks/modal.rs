//! Purpose: What is open inside the task list — a bar, a menu, a dialog, or
//! nothing — as one enum, so two can never be open at once, and the state
//! each one keeps: its text, its cursor, its focus and its frozen targets.
//!
//! The keys that move them live in `popup`; the invariants live here, in
//! constructors and private fields.

mod bars;
mod dialogs;
mod menu;

use crate::models::MaskPrompt;
use crate::scroll::ScrollTop;

use super::notice::{Failure, Notice};
use super::{ProcessKey, TaskList};

pub use bars::{Found, PidQuery, SearchBar, TextBar};
pub use dialogs::{ConfirmButton, FormFocus, KillConfirm, NiceConfirm, PriorityForm, ReniceButton};
pub use menu::{MenuCursor, TaskMenu};

/// What the task list shows on top of the table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Modal {
    #[default]
    Browse,
    Help(ScrollTop),
    Info(ProcessKey),
    /// Network adapter & diagnostics details dialog (`F3` / `Enter` on Network tab).
    NetInfo,
    Filter(TextBar),
    Search(SearchBar),
    PidJump(PidQuery),
    Menu(TaskMenu),
    Mask(MaskPrompt),
    Confirm(KillConfirm),
    Priority(PriorityForm),
    /// `[` asks before lowering a priority a user cannot raise again.
    Renice(NiceConfirm),
    /// One line per target a delivery failed for, after the sentence for
    /// the targets it reached, when it reached any.
    Error {
        done: Option<Notice>,
        failures: Vec<Failure>,
    },
}

impl TaskList {
    #[must_use]
    pub const fn modal(&self) -> &Modal {
        &self.modal
    }

    pub const fn modal_mut(&mut self) -> &mut Modal {
        &mut self.modal
    }

    pub fn set_modal(&mut self, modal: Modal) {
        self.modal = modal;
    }

    /// Back to the table, closing whatever was open on it.
    pub fn close_modal(&mut self) {
        self.modal = Modal::Browse;
    }
}
