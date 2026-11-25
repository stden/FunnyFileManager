//! Purpose: One table cell of the task list — each column's text for a
//! process, the tree guides and sideways scroll of the last column — and
//! the colour a row is drawn in.

use ratatui::style::Style;

use crate::models::{
    ProcessKind, ProcessRow, ProcessSample, ProcessState, Sibling, TaskList, TreeLine,
};

use super::super::super::panel::{clip_name, pad_cells, right_cells, Mark};
use super::super::super::{palette, printable_name, skip_cells, Cells};
use super::super::format;
use super::{data, Column};

/// The cells one tree level indents by: `├─ `, `└─ ` or `│  `.
const GUIDE: usize = 3;

/// The text of `column` for `process`, exactly `width` cells.
pub(super) fn cell(
    list: &TaskList,
    process: &ProcessRow,
    line: TreeLine,
    column: Column,
    width: Cells,
) -> String {
    let sample = process.sample();
    let right = |text: &str| right_cells(text, width);
    match column {
        Column::Pid => right(&sample.key.pid().to_string()),
        Column::User => pad_cells(&clip_name(&format::owner(&sample.owner), width), width),
        Column::Priority => right(&format::priority(sample.priority).0),
        Column::Nice => right(&format::priority(sample.priority).1),
        Column::Virtual => right(&format::size(sample.memory.map(|m| m.virtual_size))),
        Column::Resident => right(&format::size(sample.memory.map(|m| m.resident))),
        Column::Shared => right(&format::size(sample.memory.and_then(|m| m.shared))),
        Column::State => pad_cells(&sample.state.letter().to_string(), width),
        Column::Cpu => right(&format::cpu_usage(process.cpu())),
        Column::Memory => right(&format::memory_share(process.memory_share())),
        Column::Time => right(&format::cpu_time(sample.cpu_time)),
        Column::Command => command(list, sample, line, width),
    }
}

/// The last column: the tree guides and the command line or the name,
/// scrolled sideways and cut with Far's `}`.
fn command(list: &TaskList, sample: &ProcessSample, line: TreeLine, width: Cells) -> String {
    let scroll = Cells::new(list.command_scroll().cells());
    // Guides past the far edge are never seen; a chain thousands deep
    // costs no more than one that fills the column.
    let visible_levels = scroll.get().saturating_add(width.get()) / GUIDE + 1;
    let text = printable_name(list.command_view().text(sample));
    let full = format!("{}{text}", guides(line, visible_levels));
    let (shown, _) = skip_cells(&full, scroll);
    pad_cells(&clip_name(shown, width), width)
}

/// The tree guides before a row's text: `│  ` for each ancestor level that
/// continues, then `├─ ` or `└─ `; nothing for a root or the flat list. At
/// most `levels` of them.
fn guides(line: TreeLine, levels: usize) -> String {
    let TreeLine::Node { depth, sibling, .. } = line else {
        return String::new();
    };
    let Some(parents) = depth.checked_sub(1) else {
        return String::new();
    };
    let mut drawn: String = (0..parents)
        .take(levels)
        .map(|column| {
            if line.continues(column) {
                "│  "
            } else {
                "   "
            }
        })
        .collect();
    drawn.push_str(match sibling {
        Sibling::Middle => "├─ ",
        Sibling::Last => "└─ ",
    });
    drawn
}

/// How a process's row is coloured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Tone {
    Plain,
    /// A kernel thread, shown by `K`: dimmed throughout.
    Kernel,
    /// Marked: yellow throughout.
    Marked,
}

impl Tone {
    pub(super) const fn of(process: &ProcessRow, mark: Mark) -> Self {
        match (mark, process.sample().kind) {
            (Mark::Marked, _) => Self::Marked,
            (Mark::Unmarked, ProcessKind::KernelThread) => Self::Kernel,
            (Mark::Unmarked, ProcessKind::User) => Self::Plain,
        }
    }

    pub(super) fn style(self, column: Column, state: ProcessState) -> Style {
        let alert = matches!(state, ProcessState::DiskSleep | ProcessState::Zombie);
        match (self, column) {
            (Self::Marked, _) => Style::default().fg(palette::MARKED),
            (Self::Plain | Self::Kernel, Column::State) if alert => {
                Style::default().fg(palette::STATE_ALERT)
            }
            (Self::Kernel, _) => Style::default().fg(palette::HIDDEN),
            (Self::Plain, Column::Command) => Style::default().fg(palette::FILE),
            (Self::Plain, _) => data(),
        }
    }
}
