//! Purpose: Draws the Things catalog dialog (F9 Commands): the one line it shows
//! while the read is out or has failed, and the machine list with its marks,
//! cursor and filter.

use ratatui::Frame;

use super::cells::printable_name;
use super::dialog::{self, Button, Ground, Row};
use super::menu_dialog::choices;
use crate::models::things::{ThingsCatalog, ThingsDialog, ThingsState};
use crate::mouse::DialogMouse;
use crate::things::Machine;

const TITLE: &str = "Things catalog";

pub(super) fn render(f: &mut Frame<'_>, tracker: &mut DialogMouse, dialog: &ThingsDialog) {
    match &dialog.state {
        ThingsState::Setup(text) | ThingsState::Failed(text) => message(f, tracker, text),
        ThingsState::Queued(_) | ThingsState::Loading => {
            message(f, tracker, "Loading Things...");
        }
        ThingsState::Ready(catalog) => list(f, tracker, catalog),
    }
}

/// A message, closed by its one button.
fn message(f: &mut Frame<'_>, tracker: &mut DialogMouse, text: &str) {
    dialog::render_tracked(
        f,
        tracker,
        TITLE,
        Ground::Prompt,
        &[Row::Text(text)],
        &[Button::new("&Close").default_button().focused(true)],
    );
}

fn list(f: &mut Frame<'_>, tracker: &mut DialogMouse, catalog: &ThingsCatalog) {
    let shown = catalog.shown();
    if shown.is_empty() {
        let text = if catalog.machine_count() == 0 {
            "Things listed no machines"
        } else {
            "No machine matches the filter"
        };
        return message(f, tracker, text);
    }
    // The list has no buttons for the mouse to find, so the last message's are dropped.
    tracker.clear();
    let labels: Vec<String> = shown
        .into_iter()
        .map(|(marked, machine)| row_label(marked, machine))
        .collect();
    choices::render(f, &title(catalog), &labels, catalog.cursor_index());
}

/// The row of one machine: its mark, then name, host, OS, environment and state.
/// The columns are as narrow as a common name allows, since the list is drawn
/// at most 80 columns wide and a row cut short loses its state. What the server
/// sent is made printable first, so no control character reaches the terminal.
fn row_label(marked: bool, machine: &Machine) -> String {
    let mark = if marked { '*' } else { ' ' };
    let name = printable_name(&machine.name);
    let host = printable_name(machine.host());
    let os = printable_name(&machine.os()).into_owned();
    let environment = printable_name(machine.environment.as_deref().unwrap_or_default());
    let state = machine.status.label();
    format!("{mark} {name:<16} {host:<22} {os:<12} {environment:<8} {state}")
}

/// The frame's title: the filter typed so far, and how many machines the list
/// shows of the server's total when it has more than were read.
fn title(catalog: &ThingsCatalog) -> String {
    let filter = match catalog.filter() {
        "" => String::new(),
        text => format!(" | filter: {text}"),
    };
    match catalog.unshown_total() {
        Some(total) => {
            let count = catalog.machine_count();
            format!("{TITLE} (showing {count} of {total}){filter}")
        }
        None => format!("{TITLE}{filter}"),
    }
}

#[cfg(test)]
mod tests;
