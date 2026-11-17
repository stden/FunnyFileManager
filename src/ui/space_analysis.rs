//! Purpose: Draws the Space analysis dialog (Commands): the folder and its total,
//! what can be reclaimed safely, then one line per entry — size, share, name and
//! the verdict with its reason — the cursor's row marked, and the note on limits.

use ratatui::Frame;

use super::dialog::{self, Ground, Row};
use crate::models::space_analysis::{SpaceAnalysisDialog, SpaceRow};
use crate::models::FileSize;
use crate::status::PathMessage;

const TITLE: &str = "Space analysis";
const KEYS: &str = "Enter: select in the panel    Esc: close";

pub(super) fn render(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    dialog: &SpaceAnalysisDialog,
) {
    let folder = PathMessage::quoting("Folder: ", crate::path_display::label(&dialog.folder), "");
    let totals = format!(
        "Total {}    Reclaimable safely {}",
        dialog.total, dialog.reclaimable
    );
    let lines: Vec<String> = dialog
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| line(index == dialog.cursor, row))
        .collect();
    let mut rows = vec![Row::Path(&folder), Row::Plain(&totals), Row::Divider];
    rows.extend(lines.iter().map(|line| Row::Plain(line.as_str())));
    rows.push(Row::Divider);
    rows.extend((!dialog.note.is_empty()).then_some(Row::Plain(&dialog.note)));
    rows.push(Row::Plain(KEYS));
    let close = [dialog::Button::new("&Close").default_button().focused(true)];
    dialog::render_tracked(f, tracker, TITLE, Ground::Prompt, &rows, &close);
}

/// One entry: the cursor's mark, size, share, name, then the verdict and why.
fn line(selected: bool, row: &SpaceRow) -> String {
    let mark = if selected { '>' } else { ' ' };
    let size = FileSize::from_bytes(row.bytes).to_string();
    let name = crate::ui::printable_name(&row.name);
    let verdict = format!("{} {}", row.verdict.label(), row.verdict.reason());
    format!("{mark} {size:>10}  {:>3}%  {name}  {verdict}", row.share)
}

#[cfg(test)]
mod tests;
