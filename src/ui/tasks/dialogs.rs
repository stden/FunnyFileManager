//! Purpose: What the task list opens over its table — the kill confirmation,
//! the Priority form, the confirmation `[` asks for, process/network
//! information, the Error dialog, the Select/Deselect mask, its menus — and
//! the help page drawn in the table's place.
//!
//! Every line naming a process is drawn as plain text: a process names
//! itself, so an `&` in its name is shown, never taken for a hotkey mark.

mod confirm;
mod info;
mod net_info;

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

use crate::action::tasks::HELP;
use crate::models::{Failure, FormFocus, Modal, Notice, PriorityForm, Target, TaskList};
use crate::mouse::DialogMouse;
use crate::scroll::ScrollTop;

use super::super::dialog::{preview_lines, render_tracked, Button, Ground, Row};
use super::super::overlays::render_mask_prompt;
use super::super::panel::pad_cells;
use super::super::{palette, printable_name, Cells};
use super::menus::MenuView;
use confirm::{kill, renice};

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
        Modal::NetInfo => net_info::net_info(f, tracker, list),
        Modal::Error { done, failures } => error(f, tracker, *done, failures),
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
        Row::field("Nice:", value, form.focus() == FormFocus::Field, true),
        Row::Label("-20 runs first, 19 runs last"),
    ];
    let buttons = [
        Button::new("&Set")
            .default_button()
            .enabled(form.niceness().is_some())
            .focused(form.focus() == FormFocus::Set),
        Button::new("&Cancel").focused(form.focus() == FormFocus::Cancel),
    ];
    if let Some(caret) = render_tracked(f, tracker, "Priority", Ground::Prompt, &rows, &buttons) {
        f.set_cursor_position(caret);
    }
}

/// The Error dialog: what was done, when anything was, then one line per
/// target a delivery failed for.
fn error(f: &mut Frame<'_>, tracker: &mut DialogMouse, done: Option<Notice>, failures: &[Failure]) {
    let mut lines: Vec<String> = done.iter().map(ToString::to_string).collect();
    preview_lines(
        failures,
        |failure| printable_name(&failure.to_string()).into_owned(),
        &mut lines,
    );
    let rows: Vec<Row<'_>> = lines.iter().map(|line| Row::Plain(line)).collect();
    let buttons = [Button::new("OK").focused(true)];
    render_tracked(f, tracker, "Error", Ground::Warning, &rows, &buttons);
}

/// A target as a dialog names it: `4242 firefox`.
fn who(target: &Target) -> String {
    format!("{} {}", target.key().pid(), printable_name(target.name()))
}
