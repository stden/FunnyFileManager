//! Purpose: Changing the nice value — the Priority form, the `]` and `[`
//! steps, the confirmation `[` asks for when FFM is not root — and applying
//! the plan, recording every value the kernel took.

use super::{ask, finish, freeze};
use crate::models::{
    App, Attempt, Failure, Modal, NiceConfirm, NicePlan, NiceStep, Notice, Operation,
    PriorityChange, PriorityForm, ProcessHost, Renice, ReniceButton, TaskList,
};
use crate::prompt::Privilege;

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
/// once — except `[` on Unix nice when FFM is not root, which asks first,
/// because only root can raise a Unix nice priority back again.
pub fn step_nice(app: &mut App, step: NiceStep) {
    let host = app.task_host.host();
    let Some(list) = app.task_list_mut() else {
        return;
    };
    let plan = list.plan_step(step);
    let irreversible_unix_lower = plan
        .changes()
        .iter()
        .any(|c| matches!(c.change(), PriorityChange::Nice(_)));
    match (step, list.privilege()) {
        (NiceStep::Lower, Privilege::User)
            if plan.refusal().is_none() && irreversible_unix_lower =>
        {
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

/// Applies `plan`, or says why it changes nothing. Every accepted change is
/// recorded, so the next step starts from what the kernel took.
fn renice(host: Option<&dyn ProcessHost>, list: &mut TaskList, plan: &NicePlan) {
    if let Some(refusal) = plan.refusal() {
        list.set_notice(Notice::Refused(refusal));
        return;
    }
    let mut failures = Vec::new();
    for planned in plan.changes() {
        let key = planned.target().key();
        match ask(host, |host| host.set_priority(key, planned.change())) {
            Ok(()) => list.record_priority(key, planned.change()),
            Err(error) => {
                let attempt = Attempt::Renice(planned.direction());
                failures.push(Failure::new(planned.target().clone(), attempt, error));
            }
        }
    }
    let count = plan.changes().len().saturating_sub(failures.len());
    let done = match plan.renice() {
        Renice::Step(step) => Notice::Stepped { step, count },
        Renice::Set(nice) => Notice::Reniced { count, nice },
    };
    finish(list, failures, done, count);
}
