//! Purpose: The task list's two warnings before it touches a process — the
//! kill confirmation every signal goes through and the confirmation `[`
//! asks before lowering a priority — with the lines naming what they target.

use ratatui::Frame;

use crate::models::{
    ConfirmButton, ControlRefusal, KillConfirm, KillOffer, NiceConfirm, PlannedChange, Relation,
    ReniceButton, Skipped, Target,
};
use crate::mouse::DialogMouse;

use super::super::super::dialog::{preview_lines, render_tracked, Button, Ground, Row};
use super::who;

/// The red confirmation every signal goes through.
pub(super) fn kill(f: &mut Frame<'_>, tracker: &mut DialogMouse, confirm: &KillConfirm) {
    let targets = confirm.targets();
    let count = targets.targets().len();
    let (title, question) = match confirm.offer() {
        KillOffer::TerminateOrKill => (
            "Terminate",
            format!("Do you wish to terminate {}?", the(count)),
        ),
        KillOffer::Send(signal) => (
            "Send signal",
            format!("Send {} to {}?", signal.name(), the(count)),
        ),
    };
    let mut lines = Vec::new();
    preview_lines(targets.targets(), who, &mut lines);
    preview_lines(targets.skipped(), skipped, &mut lines);
    lines.extend(ancestor_note(targets.targets()));
    let rows = rows(&question, &lines, None);
    render_tracked(
        f,
        tracker,
        title,
        Ground::Warning,
        &rows,
        &kill_buttons(confirm),
    );
}

/// The confirmation's buttons, the first the default, the focus where the
/// keys left it.
fn kill_buttons(confirm: &KillConfirm) -> Vec<Button<'static>> {
    let focused = confirm.focused();
    let buttons = confirm.buttons().iter().enumerate();
    buttons
        .map(|(index, &button)| {
            let drawn = Button::new(confirm_label(button)).focused(button == focused);
            if index == 0 {
                drawn.default_button()
            } else {
                drawn
            }
        })
        .collect()
}

const fn confirm_label(button: ConfirmButton) -> &'static str {
    match button {
        ConfirmButton::Terminate => "&Terminate",
        ConfirmButton::Kill => "&Kill",
        ConfirmButton::Send => "&Send",
        ConfirmButton::Cancel => "&Cancel",
    }
}

/// The confirmation `[` asks before lowering a priority only root can
/// raise again.
pub(super) fn renice(f: &mut Frame<'_>, tracker: &mut DialogMouse, confirm: &NiceConfirm) {
    let plan = confirm.plan();
    let targets: Vec<&Target> = plan.changes().iter().map(PlannedChange::target).collect();
    let question = format!("Lower the priority of {}?", the(targets.len()));
    let mut lines = Vec::new();
    preview_lines(&targets, |target| who(target), &mut lines);
    preview_lines(plan.skipped(), skipped, &mut lines);
    let rows = rows(&question, &lines, Some("Only root can raise it again."));
    let focused = confirm.focused();
    let buttons = [
        Button::new("&Renice")
            .default_button()
            .focused(focused == ReniceButton::Renice),
        Button::new("&Cancel").focused(focused == ReniceButton::Cancel),
    ];
    render_tracked(
        f,
        tracker,
        "Lower priority",
        Ground::Warning,
        &rows,
        &buttons,
    );
}

/// A question, then `lines` naming processes, then an optional closing
/// sentence.
fn rows<'a>(question: &'a str, lines: &'a [String], closing: Option<&'a str>) -> Vec<Row<'a>> {
    let named = lines.iter().map(|line| Row::Plain(line));
    std::iter::once(Row::Text(question))
        .chain(named)
        .chain(closing.map(Row::Text))
        .collect()
}

/// `the process` or `the 3 processes`.
fn the(count: usize) -> String {
    match count {
        1 => "the process".to_owned(),
        count => format!("the {count} processes"),
    }
}

/// A refused target: `Skipped: 1 init (init)`; FFM itself by that name.
fn skipped(skipped: &Skipped) -> String {
    let target = skipped.target();
    let why = match skipped.refusal() {
        ControlRefusal::OwnProcess => return "Skipped: FFM itself".to_owned(),
        ControlRefusal::Init => "init",
        ControlRefusal::KernelThread => "kernel thread",
        ControlRefusal::Unverifiable => "identity unknown",
        ControlRefusal::AtLimit(_) | ControlRefusal::ClassAtLimit => "at its limit",
        ControlRefusal::RealtimeClass => "realtime",
        ControlRefusal::UnknownPriority => "priority unknown",
        ControlRefusal::NoTarget => "no process",
    };
    format!("Skipped: {} ({why})", who(target))
}

/// The warning that ending a process FFM runs inside ends this session,
/// naming the first such target.
fn ancestor_note(targets: &[Target]) -> Option<String> {
    let ancestor = targets
        .iter()
        .find(|target| target.relation() == Relation::Ancestor)?;
    Some(format!(
        "FFM runs inside {}; ending it ends this session.",
        who(ancestor)
    ))
}
