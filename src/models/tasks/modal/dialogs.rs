//! Purpose: The dialogs with a focus to move — the kill confirmation, the
//! priority form and the confirmation `[` asks for — each keeping what it
//! froze and a focus that is always on one of its own buttons.

mod kill_confirm;
mod nice_confirm;
mod priority_form;

pub use kill_confirm::{ConfirmButton, KillConfirm};
pub use nice_confirm::{NiceConfirm, ReniceButton};
pub use priority_form::{FormFocus, PriorityForm};
