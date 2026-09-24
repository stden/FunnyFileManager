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

mod refused_targets;
mod terminating;
