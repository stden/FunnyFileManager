//! Canonical effects for the main menu's extended commands.

use std::io;

use crate::{
    lineedit::LineEditor,
    menu::MenuCommand,
    models::{App, ChoiceTarget, MenuChoice, MenuDialog, Overlay},
    status::Status,
};

mod files;
pub mod process;
mod search;
pub mod settings;
pub mod user;
mod workflows;

pub fn activate(app: &mut App, entry: crate::menu::Entry) {
    app.close_overlay();
    if let Some(command) = entry.menu_command() {
        // A leftover F-key from an earlier row must not run after this
        // command has already opened its own dialog (Alt+F7 vs F7).
        app.pending_key = None;
        execute(app, command);
    } else {
        app.pending_key = entry.key();
    }
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
        MenuCommand::Compare => search::compare(app),
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
        MenuCommand::Filter
        | MenuCommand::LongNames
        | MenuCommand::TogglePanel(_)
        | MenuCommand::FolderHistory
        | MenuCommand::ViewHistory
        | MenuCommand::Screens
        | MenuCommand::Tasks
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
            super::lsp_status::open(app);
            Ok(())
        }
        MenuCommand::UserMenu | MenuCommand::Associations => user::edit(app, command),
        _ => Err(io::Error::other("Unknown tool")),
    }
}

pub fn choose(app: &mut App, target: ChoiceTarget) -> io::Result<()> {
    app.close_overlay();
    match target {
        ChoiceTarget::Directory(path) => app.get_active_panel().navigate_to(path)?,
        ChoiceTarget::File(path) => {
            crate::ops::view_path(app, path)?;
        }
        ChoiceTarget::Command(text) => {
            let line = crate::models::CommandLine {
                line: LineEditor::new(&text),
                ..Default::default()
            };
            app.set_overlay(Overlay::CommandLine(line));
        }
        ChoiceTarget::Menu(command) => execute(app, command),
        ChoiceTarget::Screen(view) => app.view = view,
        ChoiceTarget::Close => {}
        ChoiceTarget::UpdatePackages(plan) => app.package_update = Some(plan),
    }
    Ok(())
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

fn prompt(app: &mut App, command: MenuCommand, title: &str, caption: &str, value: &str) {
    app.set_overlay(Overlay::MenuDialog(Box::new(MenuDialog::Prompt {
        command,
        title: title.to_owned(),
        caption: caption.to_owned(),
        input: LineEditor::new(value),
    })));
}

fn history(app: &mut App) {
    let rows = (0..app.settings.number(crate::menu::SettingsPage::CommandLine))
        .map_while(|offset| app.history.recall(crate::history::RingIndex::new(offset)))
        .map(|text| MenuChoice {
            label: text.to_owned(),
            target: ChoiceTarget::Command(text.to_owned()),
        })
        .collect();
    choices(app, "Command history", rows);
}

fn shortcuts(app: &mut App) {
    let rows = app
        .bookmarks
        .slots()
        .iter()
        .enumerate()
        .filter_map(|(slot, path)| {
            path.as_ref().map(|path| MenuChoice {
                label: format!("{slot}: {}", path.display()),
                target: ChoiceTarget::Directory(path.clone()),
            })
        })
        .collect();
    choices(app, "Folder shortcuts — use :bookmark set N to add", rows);
}

fn choices(app: &mut App, title: &str, rows: Vec<MenuChoice>) {
    app.set_overlay(Overlay::MenuDialog(Box::new(MenuDialog::Choices {
        title: title.to_owned(),
        rows,
        selected: 0,
    })));
}

#[cfg(test)]
mod tests;
