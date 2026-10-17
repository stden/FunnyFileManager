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

/// Whether `entry` is marked in `panel`.
pub(super) fn mark_of(panel: &Panel, entry: &DirEntry) -> Mark {
    if panel.marked.contains(entry.name.as_str()) {
        Mark::Marked
    } else {
        Mark::Unmarked
    }
}

/// The colour Far gives a row, in the order its rules override each other.
///
/// Marked first — a mark is the user's own statement about the row and must
/// survive whatever the row is. Hidden next, because a dimmed row is dimmed
/// whether it is a folder, a program or a file. Only then does the entry's own
/// nature get a say.
pub(super) fn entry_colour(panel: &Panel, entry: &DirEntry, mark: Mark) -> Color {
    if mark == Mark::Marked {
        return palette::MARKED;
    }
    if panel.is_hidden(&entry.name) {
        return palette::HIDDEN;
    }
    if entry.kind.is_dir() {
        return if panel.is_dirty_repo(&entry.name) {
            palette::DIRTY_REPO
        } else {
            palette::DIRECTORY
        };
    }
    if entry.executable {
        return palette::EXECUTABLE;
    }
    palette::FILE
}

/// The style for a row's name: its colour, bold where Far bolds it.
pub(super) fn entry_style(panel: &Panel, entry: &DirEntry, mark: Mark) -> Style {
    let style = Style::default().fg(entry_colour(panel, entry, mark));
    if panel.is_hidden(&entry.name) {
        return style;
    }
    if mark == Mark::Marked || entry.kind.is_dir() {
        return style.add_modifier(Modifier::BOLD);
    }
    style
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
