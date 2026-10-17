//! Purpose: Draws the Install MCP server dialog (F9 Commands): the server, the
//! scope radio, one checkbox per client, the scope's notes, and the Register /
//! Cancel buttons.
//!
//! Reads the dialog's state and nothing else: whether a client is installed
//! and what its config says was surveyed when the dialog opened.

use ratatui::Frame;

use super::dialog::{self, Button, Ground, Row};
use crate::mcp_clients::{self, Scope};
use crate::models::{Mark, McpInstallDialog, McpInstallField, McpInstallProgress};
use crate::status::PathMessage;

/// The dialog's title, which is the menu row's label without its hotkey.
const TITLE: &str = "Install MCP server";

pub(super) fn render(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    dialog: &McpInstallDialog,
) {
    let server = mcp_clients::server_lines(&dialog.server);
    let folder = folder_line(dialog);
    let labels: Vec<String> = dialog
        .rows
        .iter()
        .map(|row| mcp_clients::row_label(&row.status))
        .collect();
    let notes = mcp_clients::notes(dialog.scope);
    let scopes = Scope::ALL.map(Scope::label);
    // Paths and notes are drawn as they are: `&` in `/srv/x/R&D` is a letter,
    // not a hotkey marker. A path too long for its row keeps its name.
    let mut rows: Vec<Row<'_>> = server.iter().map(Row::Path).collect();
    rows.extend([
        Row::Divider,
        Row::Radio {
            caption: "Scope:",
            options: &scopes,
            selected: dialog.scope_index(),
            focused: dialog.field == McpInstallField::Scope,
        },
        Row::Path(&folder),
        Row::Divider,
    ]);
    rows.extend(client_rows(dialog, &labels));
    // Notes are static per scope, so ticking a box never resizes the dialog.
    // While Register runs, what it waits for takes their place: a status line
    // under a dialog this size is mostly hidden.
    let shown: Vec<&str> = match &dialog.progress {
        McpInstallProgress::Choosing => notes.iter().map(String::as_str).collect(),
        McpInstallProgress::Registering(message) => vec![message],
    };
    rows.extend((!shown.is_empty()).then_some(Row::Divider));
    rows.extend(shown.into_iter().map(Row::Plain));
    dialog::render_tracked(f, tracker, TITLE, Ground::Prompt, &rows, &buttons(dialog));
}

/// The folder Current folder would register in, or why there is none.
fn folder_line(dialog: &McpInstallDialog) -> PathMessage<'_> {
    dialog.folder.as_ref().map_or_else(
        || PathMessage::from("Current folder: the active panel is not a local folder".to_owned()),
        |folder| {
            PathMessage::quoting(
                "Current folder: ",
                crate::path_display::label(folder.as_path()),
                "",
            )
        },
    )
}

/// One checkbox per client. The widget takes a `bool`; the dialog keeps a
/// [`Mark`], and this is where one becomes the other.
fn client_rows<'a>(dialog: &'a McpInstallDialog, labels: &'a [String]) -> Vec<Row<'a>> {
    dialog
        .rows
        .iter()
        .zip(labels)
        .map(|(row, label)| Row::Checkbox {
            label,
            checked: row.mark == Mark::Checked,
            focused: dialog.field == McpInstallField::Client(row.status.client),
            enabled: dialog.is_enabled(row),
        })
        .collect()
}

/// Register is the default while there is a server to register; without one
/// it is dimmed and Cancel takes its place.
fn buttons(dialog: &McpInstallDialog) -> [Button<'static>; 2] {
    let register = Button::new("Register")
        .enabled(dialog.can_register())
        .focused(dialog.field == McpInstallField::Register);
    let cancel = Button::new("Cancel").focused(dialog.field == McpInstallField::Cancel);
    if dialog.can_register() {
        [register.default_button(), cancel]
    } else {
        [register, cancel.default_button()]
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_paths;
