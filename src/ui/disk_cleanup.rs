//! Purpose: Draws the Disk Cleanup checklist dialog (`:cleanup`).
//!
//! Each candidate is a checkbox row. Rows that do not exist on disk are dimmed
//! and non-interactive, matching Far's own pattern for disabled controls.

use ratatui::Frame;

use super::dialog::{self, Button, Ground, Row};
use crate::models::{CleanupField, DiskCleanupDialog};

pub(super) fn render(
    f: &mut Frame,
    tracker: &mut crate::mouse::DialogMouse,
    dialog_state: &DiskCleanupDialog,
) {
    // Build one label string per candidate, owning the String so Row can
    // borrow it as &str within the same scope.
    let labels: Vec<String> = dialog_state
        .candidates
        .iter()
        .map(|c| {
            let size = match c.size {
                Some(s) => s.to_string(),
                None if c.exists => "measuring\u{2026}".to_owned(),
                None => "not found".to_owned(),
            };
            format!("{}  [{}]", c.label, size)
        })
        .collect();

    let mut rows: Vec<Row> = labels
        .iter()
        .enumerate()
        .map(|(idx, label)| Row::Checkbox {
            label: label.as_str(),
            checked: dialog_state.candidates[idx].checked,
            focused: dialog_state.field == CleanupField::Item(idx),
            enabled: dialog_state.candidates[idx].exists,
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

#[cfg(test)]
mod tests;
