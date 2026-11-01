//! Purpose: The dialogs over the selected vault entry: its password, the
//! one place outside the edit form an entry password is drawn, and the
//! delete confirmation.

use ratatui::Frame;

use super::super::dialog;
use crate::kdbx::Vault;

pub(super) fn render_password(
    f: &mut Frame<'_>,
    name: &str,
    vault: &Vault,
    tracker: &mut crate::mouse::DialogMouse,
) {
    let entry = vault.selected_entry();
    let title = entry.map_or("", |entry| entry.title.as_str());
    let username = entry.map_or("", |entry| entry.username.as_str());
    let password = vault.revealed().unwrap_or_default();
    let rows = [
        dialog::Row::Label("Entry:"),
        dialog::Row::Text(title),
        dialog::Row::Label("User:"),
        dialog::Row::Text(username),
        dialog::Row::Label("Password:"),
        dialog::Row::input(password, false, false),
        dialog::Row::Text("Enter Copy  ·  Esc Close"),
    ];
    let buttons = [
        dialog::Button::new("Copy").default_button(),
        dialog::Button::new("Close"),
    ];
    dialog::render_tracked(
        f,
        tracker,
        &format!("KeePass password: {name}"),
        dialog::Ground::Prompt,
        &rows,
        &buttons,
    );
}

pub(super) fn render_delete(
    f: &mut Frame<'_>,
    name: &str,
    vault: &Vault,
    tracker: &mut crate::mouse::DialogMouse,
) {
    let title = vault
        .selected_entry()
        .map_or("", |entry| entry.title.as_str());
    let rows = [
        dialog::Row::Text("Delete this KeePass entry?"),
        dialog::Row::Label(title),
        dialog::Row::Text("The database will be backed up before writing."),
    ];
    let buttons = [
        dialog::Button::new("Delete").default_button(),
        dialog::Button::new("Cancel"),
    ];
    dialog::render_tracked(
        f,
        tracker,
        &format!("Delete from: {name}"),
        dialog::Ground::Prompt,
        &rows,
        &buttons,
    );
}
