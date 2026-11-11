//! Purpose: Describe files (`Ctrl+Z`): prompt for a description and write it
//! to the directory's description file, Far's `Descript.ion`.
//!
//! The marked entries are taken in turn, one prompt each, as Far does; with
//! nothing marked the entry under the cursor is the one. The panel is
//! re-listed after every write so the Descriptions columns show the new text
//! at once.

use crate::description;
use crate::models::{App, DescribePrompt, Overlay};
use crate::status::Status;

/// Ctrl+Z: open the Describe files dialog for the marked entries, else the
/// one under the cursor.
pub fn open(app: &mut App) {
    let names = app.get_active_panel().action_entry_names();
    if names.is_empty() {
        // `..` or an empty panel: nothing Far would describe either. Said
        // aloud, or the key reads as dead.
        app.set_status(Status::CommandOutput("Nothing to describe".to_owned()));
        return;
    }
    prompt_next(app, names);
}

/// Show the prompt for the first of `names`, offering the description it
/// already has; an empty queue is the end of the dialog.
fn prompt_next(app: &mut App, mut names: Vec<String>) {
    if names.is_empty() {
        app.close_overlay();
        return;
    }
    let name = names.remove(0);
    let existing = app
        .get_active_panel()
        .entries
        .iter()
        .find(|entry| entry.name == name)
        .and_then(|entry| entry.description.clone())
        .unwrap_or_default();
    app.set_overlay(Overlay::Describe(DescribePrompt::new(
        name, &existing, names,
    )));
}

/// Enter on OK: write the description, re-list, and go on to the next
/// entry in the queue.
pub fn accept(app: &mut App) {
    let Overlay::Describe(prompt) = &app.overlay else {
        return;
    };
    let name = prompt.name().to_owned();
    let text = prompt.text().to_owned();
    let rest = prompt.pending().to_vec();
    let panel = app.get_active_panel();
    let written =
        description::set_description(&panel.path, panel.active_description_names(), &name, &text);
    if let Err(error) = written {
        app.close_overlay();
        app.set_status(Status::Error(format!("Cannot describe {name}: {error}")));
        return;
    }
    // Far unmarks each file as it is described: a second Ctrl+Z then asks
    // only about the ones Esc left behind, not the whole queue again.
    panel.marked.remove(name.as_str());
    // The line is written; a listing that cannot be re-read keeps its old rows
    // until the next refresh reports the trouble.
    drop(panel.load_entries());
    app.set_status(Status::CommandOutput(report(&name, &text)));
    prompt_next(app, rest);
}

/// What the status bar says once the line is written.
fn report(name: &str, text: &str) -> String {
    if text.trim().is_empty() {
        format!("Description of {name} removed")
    } else {
        format!("Described {name}")
    }
}

#[cfg(test)]
mod tests;
