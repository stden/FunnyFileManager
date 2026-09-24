//! Purpose: What the task list opens over its table — the kill confirmation,
//! the Priority form, the confirmation `[` asks for, process information,
//! the Error dialog, the Select/Deselect mask, its menus — and the help
//! page drawn in the table's place.
//!
//! Every line naming a process is drawn as plain text: a process names
//! itself, so an `&` in its name is shown, never taken for a hotkey mark.

mod info;

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

use crate::action::tasks::HELP;
use crate::models::{
    ConfirmButton, ControlRefusal, Failure, FormFocus, KillConfirm, KillOffer, Modal, NiceConfirm,
    PlannedChange, PriorityForm, Relation, ReniceButton, Skipped, Target, TaskList,
};
use crate::mouse::DialogMouse;
use crate::scroll::ScrollTop;

use super::super::describe::caret_column;
use super::super::dialog::{preview_lines, render_tracked, Button, Ground, Row};
use super::super::overlays::render_mask_prompt;
use super::super::panel::pad_cells;
use super::super::{palette, printable_name, Cells};
use super::menus::MenuView;

/// Draws whatever is open over the table, `body` being the table's rows.
/// A dialog records where it landed for the next click; anything else
/// leaves no record, so a click cannot land on a dialog no longer drawn.
pub(super) fn render(f: &mut Frame<'_>, tracker: &mut DialogMouse, list: &TaskList, body: Rect) {
    match list.modal() {
        Modal::Browse | Modal::Filter(_) | Modal::Search(_) | Modal::PidJump(_) => tracker.clear(),
        Modal::Help(top) => {
            help(f, body, *top);
            tracker.clear();
        }
        Modal::Menu(menu) => {
            MenuView::of(list, menu).render(f);
            tracker.clear();
        }
        Modal::Info(key) => info::info(f, tracker, list, *key),
        Modal::Error(failures) => error(f, tracker, failures),
        Modal::Mask(prompt) => render_mask_prompt(f, tracker, prompt),
        Modal::Confirm(confirm) => kill(f, tracker, confirm),
        Modal::Priority(form) => priority(f, tracker, form),
        Modal::Renice(confirm) => renice(f, tracker, confirm),
    }
}

/// The help page, over the table's rows, scrolled to `top`.
fn help(f: &mut Frame<'_>, body: Rect, top: ScrollTop) {
    let width = HELP
        .iter()
        .map(|(keys, _)| Cells::of(keys))
        .max()
        .unwrap_or(Cells::ZERO);
    let lines: Vec<Line<'static>> = HELP
        .iter()
        .skip(top.lines())
        .take(usize::from(body.height))
        .map(|(keys, does)| Line::from(format!(" {}  {does}", pad_cells(keys, width))))
        .collect();
    let style = Style::default().fg(palette::HELP_FG).bg(palette::HELP_BG);
    f.render_widget(Clear, body);
    f.render_widget(Paragraph::new(lines).style(style), body);
}

/// The red confirmation every signal goes through.
fn kill(f: &mut Frame<'_>, tracker: &mut DialogMouse, confirm: &KillConfirm) {
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
fn renice(f: &mut Frame<'_>, tracker: &mut DialogMouse, confirm: &NiceConfirm) {
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

/// The F7 form: a nice value for the frozen targets.
fn priority(f: &mut Frame<'_>, tracker: &mut DialogMouse, form: &PriorityForm) {
    let targets = form.targets().targets();
    let named = match targets {
        [only] => who(only),
        many => format!("{} processes", many.len()),
    };
    let value = form.value();
    let rows = [
        Row::Plain(&named),
        Row::Field {
            caption: "Nice:",
            value: value.text(),
            focused: form.focus() == FormFocus::Field,
            enabled: true,
        },
        Row::Label("-20 runs first, 19 runs last"),
    ];
    let buttons = [
        Button::new("&Set")
            .default_button()
            .enabled(form.niceness().is_some())
            .focused(form.focus() == FormFocus::Set),
        Button::new("&Cancel").focused(form.focus() == FormFocus::Cancel),
    ];
    let caret = render_tracked(f, tracker, "Priority", Ground::Prompt, &rows, &buttons);
    if let Some((x, y)) = caret {
        f.set_cursor_position((caret_column(x, value.after_cursor()), y));
    }
}

/// The Error dialog: one line per target a delivery failed for.
fn error(f: &mut Frame<'_>, tracker: &mut DialogMouse, failures: &[Failure]) {
    let mut lines = Vec::new();
    preview_lines(
        failures,
        |failure| printable_name(&failure.to_string()).into_owned(),
        &mut lines,
    );
    let rows: Vec<Row<'_>> = lines.iter().map(|line| Row::Plain(line)).collect();
    let buttons = [Button::new("OK").focused(true)];
    render_tracked(f, tracker, "Error", Ground::Warning, &rows, &buttons);
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

/// A target as a dialog names it: `4242 firefox`.
fn who(target: &Target) -> String {
    format!("{} {}", target.key().pid(), printable_name(target.name()))
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
