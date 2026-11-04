//! Purpose: Tests for signalling through the task list, with a fixture
//! host that records instead of sending: confirmations that freeze their
//! targets, a signal only after a button, the refusals, and every outcome
//! told as a notice or the Error dialog.

use crate::models::{
    ConfirmButton, ControlError, ControlRefusal, KillOffer, Modal, NiceStep, Notice, Schedule,
    Signal,
};
use crate::prompt::Privilege;
use crate::test_support::processes::Control;
use crate::test_support::task_list::{key_of, named, TaskApp, OWN};

use super::{confirm_terminate, open_signal_menu, press, step_nice};

fn firefox() -> TaskApp {
    TaskApp::open(vec![named(4242, "firefox")], Privilege::User)
}

mod mixed_outcome;
mod refused_targets;
mod terminating;

#[test]
fn signal_modal_falls_back_to_terminate_confirm_when_no_menu_is_offered() {
    let mut task = firefox();
    let targets = task.list().freeze_targets(crate::models::Operation::Signal);
    assert!(matches!(
        super::signal_modal(targets, None),
        Modal::Confirm(_)
    ));
}
