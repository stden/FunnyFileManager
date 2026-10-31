//! Canonical effects for the F9 menu's commands: the rows outside the
//! families, and the panel, file and tool families a key names too.

use std::io;

use crate::action::{FileCommand, PanelCommand, ToolCommand};
use crate::{
    menu::MenuCommand,
    models::{App, MenuPrompt},
    status::Status,
};

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

/// Close the drop-down and leave `entry` for the dispatcher, which runs its
/// action through the same road a key takes — the active panel's kind asked
/// first — once the menu has left the screen.
pub fn activate(app: &mut App, entry: crate::menu::Entry) {
    app.close_overlay();
    app.pending_row = Some(entry);
}

pub fn execute(app: &mut App, command: MenuCommand) {
    if let Err(error) = execute_result(app, command) {
        app.set_status(Status::Error(error.to_string()));
    }
}

fn execute_result(app: &mut App, command: MenuCommand) -> io::Result<()> {
    match command {
        MenuCommand::Compare => compare::compare(app),
        MenuCommand::Shortcuts => shortcuts(app),
        MenuCommand::Settings(page) => settings::open(app, page),
        MenuCommand::SaveSetup => settings::save(app)?,
        MenuCommand::Devices => workflows::devices(app),
        MenuCommand::Path(scope) => super::path_env::open(app, scope)?,
        MenuCommand::Performance => super::performance::open(app),
        MenuCommand::TokenUsage => super::token_usage::open(app),
        MenuCommand::UpdatePackages => super::packages::open(app),
        MenuCommand::LanguageServers => super::lsp_editor::open(app),
        MenuCommand::McpInstall => super::mcp_install::open(app),
        MenuCommand::GitPullAll => super::git_pull::queue(app)?,
        MenuCommand::UserMenu | MenuCommand::Associations => user::edit(app, command)?,
    }
    Ok(())
}

/// The Left / Right menu: how the active panel shows its listing.
pub fn execute_panel(app: &mut App, command: PanelCommand) {
    match command {
        PanelCommand::QuickView => super::quickview::toggle(app),
        PanelCommand::Tree => super::tree::toggle(app),
        PanelCommand::Sort(mode) => super::sortmenu::set(app, mode),
        PanelCommand::SortMenu => super::sortmenu::open(app),
        PanelCommand::TogglePanel(side) => super::panels::toggle(app, side),
        PanelCommand::LongNames => super::longnames::toggle(app),
    }
}

/// The Files menu rows beyond the bare F-keys: each opens its own dialog.
pub fn execute_file(app: &mut App, command: FileCommand) {
    match command {
        FileCommand::Link => super::link::open(app),
        FileCommand::Wipe => super::wipe::open(app),
        FileCommand::AddToArchive => super::archive::open_add(app),
        FileCommand::Extract => super::archive::open_extract(app),
        FileCommand::ArchiveCommands => super::archive::open_commands(app),
        FileCommand::Attributes => super::attributes::open(app),
        FileCommand::ApplyCommand => super::apply::open(app),
        FileCommand::Describe => super::describe::open(app),
    }
}

/// The Commands menu: searches, histories and the program's own windows.
pub fn execute_tool(app: &mut App, command: ToolCommand) -> io::Result<()> {
    match command {
        ToolCommand::FindFile => super::findfile::open(app),
        ToolCommand::FindFolder => search::prompt(app),
        ToolCommand::History => history(app),
        ToolCommand::VideoMode => super::videomode::toggle(app)?,
        ToolCommand::ViewHistory => workflows::view_history(app),
        ToolCommand::FolderHistory => workflows::folder_history(app),
        ToolCommand::Filter => workflows::open_filter(app),
        ToolCommand::Screens => workflows::screens(app),
        ToolCommand::Tasks => super::tasks::open(app)?,
    }
    Ok(())
}

/// Alt+F7: the dialog has a mask and a containing-text field. The menu prompt
/// has only the mask and calls [`submit`].
pub fn find_files(app: &mut App, mask: &str, containing: &str) {
    search::find(app, mask, false, containing);
}

/// The answer typed into the menu's one-line prompt.
pub fn submit(app: &mut App, purpose: MenuPrompt, text: &str) -> io::Result<()> {
    match purpose {
        MenuPrompt::FindFolder => search::find(app, text, true, ""),
        MenuPrompt::Settings(page) => settings::submit(app, page, text)?,
        MenuPrompt::Filter => workflows::filter(app, text)?,
    }
    Ok(())
}

#[cfg(test)]
mod tests;
