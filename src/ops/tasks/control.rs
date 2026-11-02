//! Purpose: What the task list does to processes, and how it tells the
//! outcome.
//!
//! It freezes the targets, opens the confirmation, sends the signal, and
//! steps or sets the nice value; the outcome is a notice, or the Error dialog
//! with one line per target that failed.
//!
//! Safety rules (spec section 8): the targets are the identities frozen when
//! the dialog opened, never whatever the cursor is on when Enter is pressed;
//! no signal goes out without a confirmation the user answered; FFM itself,
//! init and kernel threads are refused before any system call, and the host
//! refuses the first two again. Kill and renice run here, on the UI thread,
//! so they work while a sample is stuck in the kernel. Afterwards a sample is
//! due at once, so the table shows the result.

use crate::models::{
    App, Attempt, ConfirmButton, ControlError, Failure, FrozenTargets, KillConfirm, KillOffer,
    Modal, Notice, Operation, ProcessHost, Signal, Target, TaskList, TaskMenu,
};

mod priority;

pub use priority::{answer_renice, open_priority, set_nice, step_nice};

/// F8 and Del: the Terminate confirmation for the marked processes, or else
/// the cursor's; only a notice when every one of them is refused.
pub fn confirm_terminate(list: &mut TaskList) {
    let Some(targets) = freeze(list, Operation::Signal) else {
        return;
    };
    let confirm = KillConfirm::new(targets, KillOffer::TerminateOrKill);
    list.set_modal(Modal::Confirm(confirm));
}

/// F9 and `k`: the Send signal menu over the frozen targets; where the
/// platform offers no signal to choose, the Terminate confirmation.
pub fn open_signal_menu(list: &mut TaskList) {
    let Some(targets) = freeze(list, Operation::Signal) else {
        return;
    };
    let modal = TaskMenu::signal(targets.clone()).map_or_else(
        || Modal::Confirm(KillConfirm::new(targets, KillOffer::TerminateOrKill)),
        Modal::Menu,
    );
    list.set_modal(modal);
}

/// The targets `operation` has now, or `None` after a notice saying why
/// every one of them is refused.
fn freeze(list: &mut TaskList, operation: Operation) -> Option<FrozenTargets> {
    let targets = list.freeze_targets(operation);
    match targets.refusal() {
        Some(refusal) => {
            list.set_notice(Notice::Refused(refusal));
            None
        }
        None => Some(targets),
    }
}

/// A button of the kill confirmation was pressed.
///
/// Cancel closes it; a button that sends a signal closes it and delivers
/// that signal to every frozen target; a button this confirmation does not
/// have does nothing.
pub fn press(app: &mut App, button: ConfirmButton) {
    let host = app.task_host.host();
    let Some(list) = app.task_list_mut() else {
        return;
    };
    let Modal::Confirm(confirm) = list.modal() else {
        return;
    };
    if !confirm.buttons().contains(&button) {
        return;
    }
    let chosen = confirm.signal(button);
    let targets = confirm.targets().targets().to_vec();
    list.close_modal();
    if let Some(signal) = chosen {
        send(host.as_deref(), list, &targets, signal);
    }
}

/// Sends `signal` to every target and tells how it went.
fn send(host: Option<&dyn ProcessHost>, list: &mut TaskList, targets: &[Target], signal: Signal) {
    let failures: Vec<Failure> = targets
        .iter()
        .filter_map(|target| {
            let error = ask(host, |host| host.signal(target.key(), signal)).err()?;
            Some(Failure::new(target.clone(), Attempt::Signal, error))
        })
        .collect();
    let count = targets.len().saturating_sub(failures.len());
    finish(list, failures, Notice::Sent { signal, count }, count);
}

/// Asks the host, if there is one; a list opened without one can control
/// nothing.
fn ask(
    host: Option<&dyn ProcessHost>,
    act: impl FnOnce(&dyn ProcessHost) -> Result<(), ControlError>,
) -> Result<(), ControlError> {
    host.map_or(Err(ControlError::Unsupported), act)
}

/// Tells the outcome — `done` when every target took it, otherwise the Error
/// dialog, one line per failure after `done` when `succeeded` targets took
/// it — and asks for a sample at once so the table shows the result.
///
/// A mixed delivery used to show the failures alone, so a process that was
/// ended read as one that was not.
fn finish(list: &mut TaskList, failures: Vec<Failure>, done: Notice, succeeded: usize) {
    if failures.is_empty() {
        list.set_notice(done);
    } else {
        failures.iter().for_each(log);
        let done = (succeeded > 0).then_some(done);
        list.set_modal(Modal::Error { done, failures });
    }
    list.refresh_after_control();
}

/// Records an unexpected system error in the log, by PID only: a process
/// name is untrusted text.
fn log(failure: &Failure) {
    if let ControlError::Failed(kind) = failure.error() {
        crate::diagnostics::log_error(&format!(
            "task list: {:?} of PID {} failed: {kind}",
            failure.attempt(),
            failure.target().key().pid()
        ));
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_refresh;
#[cfg(test)]
mod tests_renice;
