//! Purpose: Far's selection summary — how many entries are marked and how many
//! bytes they hold, written over the divider above the panel's info row.
//!
//! Its own file because it is the one part of the panel that reads the marks
//! as a set rather than row by row.

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::{abbreviate_bytes, group_digits, head_cells, palette, Cells, FAR_BLUE};
use crate::models::Panel;

/// Far's selection summary, centred on the divider in the marked entries'
/// colour while anything is marked. The rule it covers is what Far covers:
/// with a mark on screen, how many and how big is the question being asked.
pub(super) fn render(f: &mut Frame<'_>, panel: &Panel, row: Rect) {
    let Some(summary) = selection_summary(panel, Cells::columns(row.width)) else {
        return;
    };
    let style = Style::default().fg(palette::MARKED).bg(FAR_BLUE);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(summary, style))).centered(),
        row,
    );
}

/// `" 29 bytes in 2 files "`, or `None` with nothing marked.
///
/// A folder's row carries no size of its own, so a marked folder is counted
/// rather than summed. The exact byte count, grouped as Far prints it, gives
/// way to the short form where the row is too narrow for it.
fn selection_summary(panel: &Panel, width: Cells) -> Option<String> {
    if panel.marked.is_empty() {
        return None;
    }
    let (folders, files, bytes) = panel.marked_totals();
    if folders == 0 && files == 0 {
        return None;
    }
    let mut counts = counted(files, "file");
    if folders > 0 {
        counts = format!("{counts}, {}", counted(folders, "folder"));
    }
    let exact = format!(" {} bytes in {counts} ", group_digits(bytes));
    if Cells::of(&exact) <= width {
        return Some(exact);
    }
    Some(head_cells(
        &format!(" {} in {counts} ", abbreviate_bytes(bytes)),
        width,
    ))
}

/// `count` of `noun`, in the singular for one.
fn counted(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}
