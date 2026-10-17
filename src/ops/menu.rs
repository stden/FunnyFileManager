//! Canonical effects for the main menu's extended commands.

use std::io;

use crate::{menu::MenuCommand, models::App, status::Status};

mod compare;
mod dialogs;
mod files;
pub mod process;
mod search;
pub mod settings;
pub mod user;
mod workflows;

pub use dialogs::choose;
// The children open their lists and prompts as `super::choices` / `super::prompt`.
use dialogs::{choices, history, prompt, shortcuts};

/// Run the row under the cursor of `menu`, as Enter, its letter or a click
/// does.
///
/// The Left and Right menus act on their own side, but the keys behind their
/// rows — Ctrl+L, Ctrl+R, Alt+F1/F2 — follow the active panel, so choosing
/// one of those rows makes that panel active first. A hidden panel (Ctrl+F1 /
/// Ctrl+F2) is the exception: its "Panel On/Off" row still runs, but the
/// cursor stays on the panel that is on screen.
pub fn activate_row(app: &mut App, menu: crate::menu::Menu) {
    let Some(entry) = menu.entry().copied() else {
        app.close_overlay();
        return;
    };
    if let Some(side) = menu.heading.side() {
        app.select_panel(side);
    }
    activate(app, entry);
}

pub fn activate(app: &mut App, entry: crate::menu::Entry) {
    app.close_overlay();
    if let Some(command) = entry.menu_command() {
        // A leftover F-key from an earlier row must not run after this
        // command has already opened its own dialog (Alt+F7 vs F7).
        app.pending_key = None;
        if acts_on_hidden_entries(app, command) {
            app.set_status(Status::CommandOutput(HIDDEN_ENTRIES.to_owned()));
            return;
        }
        execute(app, command);
    } else {
        app.pending_key = entry.key();
    }
}

/// What a menu row that would act on the files under a `NetBox` panel says.
const HIDDEN_ENTRIES: &str =
    "NetBox: this command works on files; press Enter on `..` to return to them";

/// Whether `command` would act on the entries of a file list that a `NetBox`
/// panel hides. A keyed row is refused by the panel itself, since its key
/// comes back through the dispatcher; a row run here must be asked the same.
fn acts_on_hidden_entries(app: &App, command: MenuCommand) -> bool {
    app.get_active_panel_ref().kind.is_netbox()
        && crate::action::menu_scope(command) == crate::action::ActionScope::Entries
}

pub fn execute(app: &mut App, command: MenuCommand) {
    if let Err(error) = execute_result(app, command) {
        app.set_status(Status::Error(error.to_string()));
    }
}

fn open_search(app: &mut App, command: MenuCommand) {
    if command == MenuCommand::FindFile {
        crate::ops::findfile::open(app);
    } else {
        search::prompt(app, command);
    }
}

fn execute_result(app: &mut App, command: MenuCommand) -> io::Result<()> {
    match command {
        MenuCommand::Sort => super::sortmenu::open(app),
        MenuCommand::FindFile | MenuCommand::FindFolder => open_search(app, command),
        MenuCommand::Tree => super::tree::toggle(app),
        MenuCommand::Compare => compare::compare(app),
        MenuCommand::Link => crate::ops::link::open(app),
        MenuCommand::Wipe => crate::ops::wipe::open(app),
        MenuCommand::Describe => crate::ops::describe::open(app),
        MenuCommand::Attributes => crate::ops::attributes::open(app),
        MenuCommand::ApplyCommand => crate::ops::apply::open(app),
        MenuCommand::QuickView => crate::ops::quickview::toggle(app),
        MenuCommand::History => history(app),
        MenuCommand::Shortcuts => shortcuts(app),
        MenuCommand::Settings(page) => settings::open(app, page),
        MenuCommand::SaveSetup => settings::save(app)?,
        MenuCommand::Tasks => super::tasks::open(app)?,
        MenuCommand::Filter
        | MenuCommand::LongNames
        | MenuCommand::TogglePanel(_)
        | MenuCommand::FolderHistory
        | MenuCommand::ViewHistory
        | MenuCommand::Screens
        | MenuCommand::Devices
        | MenuCommand::VideoMode => workflows::execute(app, command)?,
        MenuCommand::Archive => crate::ops::archive::open_add(app),
        MenuCommand::Extract => crate::ops::archive::open_extract(app),
        MenuCommand::ArchiveCommands => crate::ops::archive::open_commands(app),
        other => execute_tools(app, other)?,
    }
    Ok(())
}

fn execute_tools(app: &mut App, command: MenuCommand) -> io::Result<()> {
    match command {
        MenuCommand::Path(scope) => super::path_env::open(app, scope),
        MenuCommand::Performance => {
            super::performance::open(app);
            Ok(())
        }
        MenuCommand::UpdatePackages => {
            super::packages::open(app);
            Ok(())
        }
        MenuCommand::LanguageServers => {
            super::lsp_editor::open(app);
            Ok(())
        }
        MenuCommand::McpInstall => {
            super::mcp_install::open(app);
            Ok(())
        }
        MenuCommand::UserMenu | MenuCommand::Associations => user::edit(app, command),
        _ => Err(io::Error::other("Unknown tool")),
    }
}

/// Alt+F7: the dialog has a mask and a containing-text field. The menu prompt
/// has only the mask and calls [`submit`].
pub fn find_files(app: &mut App, mask: &str, containing: &str) {
    search::find(app, mask, false, containing);
}

pub fn submit(app: &mut App, command: MenuCommand, text: &str) -> io::Result<()> {
    match command {
        MenuCommand::FindFile => {
            search::find(app, text, false, "");
            Ok(())
        }
        MenuCommand::FindFolder => {
            search::find(app, text, true, "");
            Ok(())
        }
        MenuCommand::Settings(page) => settings::submit(app, page, text),
        MenuCommand::Filter => workflows::filter(app, text),

        _ => Err(io::Error::other("This command has no text prompt")),
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_netbox;
