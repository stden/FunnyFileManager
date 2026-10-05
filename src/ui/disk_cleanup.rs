//! Purpose: Draws the Disk Cleanup checklist dialog (`:cleanup`).
//!
//! Each candidate is a checkbox row. Rows that do not exist on disk are dimmed
//! and non-interactive, matching Far's own pattern for disabled controls.

use ratatui::Frame;

use super::dialog::{self, Button, Ground, Row};
use crate::models::{CleanupCandidate, CleanupField, CleanupKind, DiskCleanupDialog};

pub(super) fn render(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    dialog_state: &DiskCleanupDialog,
) {
    // Build one label string per candidate, owning the String so Row can
    // borrow it as &str within the same scope.
    let labels: Vec<String> = dialog_state.candidates.iter().map(row_text).collect();

    let mut rows: Vec<Row<'_>> = labels
        .iter()
        .zip(&dialog_state.candidates)
        .enumerate()
        .map(|(idx, (label, candidate))| Row::Checkbox {
            label: label.as_str(),
            checked: candidate.checked,
            focused: dialog_state.field == CleanupField::Item(idx),
            enabled: candidate.exists,
        })
        .collect();

    rows.push(Row::Divider);

    let buttons = [
        Button::new("&Clean up")
            .default_button()
            .focused(dialog_state.field == CleanupField::Run),
        Button::new("&Cancel").focused(dialog_state.field == CleanupField::Cancel),
    ];

    dialog::render_tracked(f, tracker, "Disk Cleanup", Ground::Prompt, &rows, &buttons);
}

/// A row names what it cleans, where it looks, and how much it holds.
fn row_text(candidate: &CleanupCandidate) -> String {
    let size = match candidate.size {
        Some(size) => size.to_string(),
        None if candidate.exists => "measuring\u{2026}".to_owned(),
        None => "not found".to_owned(),
    };
    let location = match candidate.kind {
        CleanupKind::HeapDumps => candidate.path.join("*.hprof"),
        CleanupKind::Path | CleanupKind::Contents => candidate.path.clone(),
    };
    let location = crate::path_display::label(&location);
    format!("{} ({location})  [{size}]", candidate.label)
}

#[cfg(test)]
mod tests;
