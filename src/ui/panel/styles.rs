//! Purpose: The colours of a panel's rows: marked, hidden, folder, program or
//! file in the order Far's rules override each other, the column rules, and
//! the cursor bar that never hides a mark.

use ratatui::style::{Color, Modifier, Style};

use super::super::palette;
use crate::models::{DirEntry, Panel};

/// Whether the user marked a row: a panel entry, or a process in the task
/// list. A marked row is yellow throughout, under the cursor bar too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::ui) enum Mark {
    Marked,
    Unmarked,
}

#[derive(Clone, Copy)]
pub(super) struct RowFacts {
    pub(super) mark: Mark,
    pub(super) name_style: Style,
    pub(super) data_style: Style,
}

/// Whether `entry` is marked in `panel`.
pub(super) fn mark_of(panel: &Panel, entry: &DirEntry) -> Mark {
    if !panel.marked.is_empty() && panel.marked.contains(entry.name.as_str()) {
        Mark::Marked
    } else {
        Mark::Unmarked
    }
}

/// Resolve mark, name style and column data style for `entry` in a single pass.
pub(super) fn row_facts(panel: &Panel, entry: &DirEntry) -> RowFacts {
    let mark = mark_of(panel, entry);
    let hidden = panel.is_hidden(&entry.name);
    let dimmed = hidden || panel.is_ignored(&entry.name);
    let is_dir = entry.kind.is_dir();
    let colour = row_colour(panel, entry, mark, hidden, dimmed, is_dir);
    let base = Style::default().fg(colour);
    let name_style = if !dimmed && (mark == Mark::Marked || is_dir) {
        base.add_modifier(Modifier::BOLD)
    } else {
        base
    };
    let whole_row = mark == Mark::Marked || dimmed || (entry.executable && !is_dir);
    let data_style = Style::default().fg(if whole_row { colour } else { palette::DATA });
    RowFacts {
        mark,
        name_style,
        data_style,
    }
}

fn row_colour(
    panel: &Panel,
    entry: &DirEntry,
    mark: Mark,
    hidden: bool,
    dimmed: bool,
    is_dir: bool,
) -> Color {
    if mark == Mark::Marked {
        palette::MARKED
    } else if hidden {
        palette::HIDDEN
    } else if dimmed {
        palette::IGNORED
    } else if is_dir {
        if panel.is_dirty_repo(&entry.name) {
            palette::DIRTY_REPO
        } else {
            palette::DIRECTORY
        }
    } else if entry.executable {
        palette::EXECUTABLE
    } else {
        palette::FILE
    }
}

/// The style for a row's name: its colour, bold where Far bolds it.
pub(super) fn entry_style(panel: &Panel, entry: &DirEntry, _mark: Mark) -> Style {
    row_facts(panel, entry).name_style
}

/// The colour of the column rules, which is the frame's.
pub(in crate::ui) fn separator_style() -> Style {
    Style::default().fg(palette::FRAME)
}

/// The style of the row under the active cursor.
pub(in crate::ui) fn cursor_style(mark: Mark) -> Style {
    Style::default()
        .bg(palette::CURSOR_BG)
        .fg(cursor_colour(mark))
        .add_modifier(Modifier::BOLD)
}

/// The text colour on the cursor bar: Far's marked yellow when the row under
/// it is marked, so the bar never hides the mark — the one thing Insert on
/// the last row changes.
pub(in crate::ui) const fn cursor_colour(mark: Mark) -> Color {
    match mark {
        Mark::Marked => palette::MARKED,
        Mark::Unmarked => palette::CURSOR_FG,
    }
}
