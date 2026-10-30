//! Purpose: Far's "File already exists" warning: the name both entries want,
//! the size and time of the new one and of the one already there, and what
//! to do about it.

use ratatui::Frame;

use super::super::dialog::{self, Ground, Row};
use super::super::{printable_name, tail_cells, Cells};
use crate::models::{ClashFocus, EntryFacts, ExistingFile};

/// The widest a path is shown before its start gives way to `…`: the file
/// name at its end is the part worth reading.
const PATH_CELLS: u16 = 64;

pub(super) fn render(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    question: &ExistingFile,
) {
    let Some(clash) = question.clash() else {
        return;
    };
    let width = f.area().width.saturating_sub(12).min(PATH_CELLS);
    let path = printable_name(&crate::path_display::label(&clash.dest)).into_owned();
    let lines = [
        heading(question.position()),
        tail_cells(&path, Cells::columns(width)),
        facts_line("New", &clash.incoming),
        facts_line("Existing", &clash.existing),
    ];
    let [heading, path, incoming, existing] = &lines;
    let focus = question.focus;
    let rows = [
        Row::Text(heading),
        Row::Text(path),
        Row::Divider,
        Row::Plain(incoming),
        Row::Plain(existing),
        Row::Divider,
        Row::checkbox(
            "R&emember choice",
            question.remember,
            focus == ClashFocus::Remember,
        ),
    ];
    let buttons = buttons(focus);
    dialog::render_tracked(f, tracker, "Warning", Ground::Warning, &rows, &buttons);
}

/// Overwrite first, as Far has it: the answer Enter gives at once.
fn buttons(focus: ClashFocus) -> [dialog::Button<'static>; 4] {
    [
        dialog::Button::new("&Overwrite")
            .default_button()
            .focused(focus == ClashFocus::Overwrite),
        dialog::Button::new("&Skip").focused(focus == ClashFocus::Skip),
        dialog::Button::new("&Keep both").focused(focus == ClashFocus::KeepBoth),
        dialog::Button::new("&Cancel").focused(focus == ClashFocus::Cancel),
    ]
}

/// The first line, counting the clashes when there is more than one.
fn heading((at, of): (usize, usize)) -> String {
    if of > 1 {
        format!("Destination already exists ({at} of {of})")
    } else {
        "Destination already exists".to_owned()
    }
}

/// `New       12,345  2026-10-05 12:00:00`: one side of the clash, the sizes
/// right-aligned so the two lines compare at a glance.
fn facts_line(label: &str, facts: &EntryFacts) -> String {
    let size = match facts.size {
        _ if facts.is_folder => "<Folder>".to_owned(),
        Some(bytes) => super::super::panel::group_digits(bytes),
        None => "?".to_owned(),
    };
    let modified = facts
        .modified
        .map_or_else(|| "?".to_owned(), crate::models::format_time);
    format!("{label:<9}{size:>16}  {modified}")
}

#[cfg(test)]
mod tests;
