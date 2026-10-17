//! Purpose: Tests for what opens on the task list: the PID query's limits,
//! menu cursors that stay on a row, the confirmation's buttons and signals,
//! the priority form's value, and every notice sentence.

use super::tests_support::{listing, process};
use super::{
    Attempt, ConfirmButton, ControlError, ControlRefusal, Failure, FormFocus, Found, FrozenTargets,
    KillConfirm, KillOffer, MenuCursor, Modal, NiceStep, Niceness, Notice, Operation, Owner, Pid,
    PidQuery, PriorityForm, SearchBar, Severity, Signal, Sort, SortKey, TaskMenu, TextBar,
    UserScope,
};
use crate::models::SortOrder;

mod tests_confirm;
mod tests_inputs;
mod tests_menus;
mod tests_notices;

fn nice(value: i32) -> Niceness {
    Niceness::try_from(value).expect("a fixture nice value")
}

fn vim() -> FrozenTargets {
    listing(vec![process(2, "vim")]).freeze_targets(Operation::Signal)
}
