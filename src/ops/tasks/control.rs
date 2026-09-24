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
    Modal, NiceConfirm, NicePlan, NiceStep, Notice, Operation, PriorityChange, PriorityForm,
    ProcessHost, Renice, ReniceButton, Signal, Target, TaskList, TaskMenu,
};
use crate::prompt::Privilege;

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

/// F7: the Priority form for the marked processes, or else the cursor's,
/// offering the first one's nice value.
pub fn open_priority(list: &mut TaskList) {
    let Some(targets) = freeze(list, Operation::Renice) else {
        return;
    };
    let offered = list.offered_nice(&targets);
    list.set_modal(Modal::Priority(PriorityForm::new(targets, offered)));
}

/// The Priority form's Set: its value for every target the form froze. While
/// the text is not a nice value, Set is disabled and the form stays open.
pub fn set_nice(app: &mut App) {
    let host = app.task_host.host();
    let Some(list) = app.task_list_mut() else {
        return;
    };
    let Modal::Priority(form) = list.modal() else {
        return;
    };
    let Some(nice) = form.niceness() else {
        return;
    };
    let plan = list.plan_set(form.targets(), nice);
    list.close_modal();
    renice(host.as_deref(), list, &plan);
}

/// `]` and `[`: one step for the marked processes, or else the cursor's, at
/// once — except `[` when FFM is not root, which asks first, because only
/// root can raise the priority again.
pub fn step_nice(app: &mut App, step: NiceStep) {
    let host = app.task_host.host();
    let Some(list) = app.task_list_mut() else {
        return;
    };
    let plan = list.plan_step(step);
    match (step, list.privilege()) {
        (NiceStep::Lower, Privilege::User) if plan.refusal().is_none() => {
            list.set_modal(Modal::Renice(NiceConfirm::new(plan)));
        }
        (NiceStep::Lower | NiceStep::Raise, _) => renice(host.as_deref(), list, &plan),
    }
}

/// The confirmation `[` asked for was answered with `button`.
pub fn answer_renice(app: &mut App, button: ReniceButton) {
    let host = app.task_host.host();
    let Some(list) = app.task_list_mut() else {
        return;
    };
    let Modal::Renice(confirm) = list.modal() else {
        return;
    };
    let plan = confirm.plan().clone();
    list.close_modal();
    match button {
        ReniceButton::Renice => renice(host.as_deref(), list, &plan),
        ReniceButton::Cancel => {}
    }
}

/// Sends `signal` to every target and tells how it went.
fn send(host: Option<&dyn ProcessHost>, list: &mut TaskList, targets: &[Target], signal: Signal) {
    let failures = targets
        .iter()
        .filter_map(|target| {
            let error = ask(host, |host| host.signal(target.key(), signal)).err()?;
            Some(Failure::new(target.clone(), Attempt::Signal, error))
        })
        .collect();
    let count = targets.len();
    finish(list, failures, Notice::Sent { signal, count });
}

/// Applies `plan`, or says why it changes nothing. Every accepted nice value
/// is recorded, so the next step starts from what the kernel took.
fn renice(host: Option<&dyn ProcessHost>, list: &mut TaskList, plan: &NicePlan) {
    if let Some(refusal) = plan.refusal() {
        list.set_notice(Notice::Refused(refusal));
        return;
    }
    let mut failures = Vec::new();
    for planned in plan.changes() {
        let key = planned.target().key();
        match ask(host, |host| host.set_priority(key, planned.change())) {
            Ok(()) => {
                if let PriorityChange::Nice(nice) = planned.change() {
                    list.record_nice(key, nice);
                }
            }
            Err(error) => {
                let attempt = Attempt::Renice(planned.direction());
                failures.push(Failure::new(planned.target().clone(), attempt, error));
            }
        }
    }
    let count = plan.changes().len();
    let done = match plan.renice() {
        Renice::Step(step) => Notice::Stepped { step, count },
        Renice::Set(nice) => Notice::Reniced { count, nice },
    };
    finish(list, failures, done);
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
/// dialog, one line per failure — and asks for a sample at once so the
/// table shows the result.
fn finish(list: &mut TaskList, failures: Vec<Failure>, done: Notice) {
    if failures.is_empty() {
        list.set_notice(done);
    } else {
        failures.iter().for_each(log);
        list.set_modal(Modal::Error(failures));
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
