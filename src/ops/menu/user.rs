//! Editable user commands and extension associations.
//!
//! Both files live in the configuration directory, beside the settings.
//! With no configuration directory (HOME unset) there is no user menu and no
//! association: a file in the folder on screen is never configuration, or
//! Enter in a cloned repository would run a command its author wrote.

use crate::{
    fileload::read_small_file,
    menu::{MenuCommand, SettingsPage},
    models::{App, ChoiceTarget, MenuChoice, Overlay},
};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// Why the user menu or the associations cannot be edited.
const NO_CONFIG_DIR: &str = "No configuration directory: set HOME to keep a user menu";

/// `filename` in the configuration directory; `None` when there is none.
fn config_path(app: &App, filename: &str) -> Option<PathBuf> {
    let settings = app.settings.persistence.file()?;
    settings.parent().map(|dir| dir.join(filename))
}

pub fn edit(app: &mut App, command: MenuCommand) -> io::Result<()> {
    let (filename, template) = match command {
        MenuCommand::UserMenu => ("user-menu.txt", "# One command per line: Label<TAB>command\n# Example: List files\tpwd\n"),
        MenuCommand::Associations => ("associations.txt", "# One association per line: extension<TAB>command\n# Use {} for the quoted selected filename.\n# Example: txt\tmy-editor {}\n"),
        _ => return Err(io::Error::other("Not an editable command file")),
    };
    let path = config_path(app, filename)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, NO_CONFIG_DIR))?;
    if let Some(parent) = path.parent() {
        crate::file_write::create_dirs_owner_only(parent)?;
    }
    if !path.exists() {
        fs::write(&path, template)?;
    }
    // What the disk held, so a save notices another program's (or another
    // instance's) write; read with the guards of every configuration file.
    let bytes = read_small_file(&path, filename)?.into_bytes();
    let mut editor = crate::editor::editor_for(app, path, &bytes)?;
    // `Label<TAB>command` is the format: Tab types the separator even when
    // no line holding one is left in the file.
    editor.format.indent = crate::models::Indent::Tabs;
    app.set_overlay(Overlay::Editor(editor));
    Ok(())
}

pub fn open(app: &mut App) -> io::Result<()> {
    let mut rows = config_path(app, "user-menu.txt")
        .map_or_else(|| Ok(Vec::new()), |path| read_pairs(&path))?
        .into_iter()
        .map(|(label, command)| MenuChoice {
            label,
            target: ChoiceTarget::Command(command),
        })
        .collect::<Vec<_>>();
    let plugin = app.settings.get(SettingsPage::Plugins);
    if !plugin.is_empty() {
        rows.push(MenuChoice {
            label: "Configured plugin command".to_owned(),
            target: ChoiceTarget::Command(plugin.to_owned()),
        });
    }
    rows.push(MenuChoice {
        label: "Edit user menu".to_owned(),
        target: ChoiceTarget::Menu(MenuCommand::UserMenu),
    });
    super::choices(app, "User menu", rows);
    Ok(())
}

pub fn associated_command(app: &App) -> io::Result<Option<String>> {
    let Some(path) = app.get_active_panel_ref().selected_path() else {
        return Ok(None);
    };
    if !path.is_file() {
        return Ok(None);
    }
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("");
    let Some(associations) = config_path(app, "associations.txt") else {
        return Ok(None);
    };
    let pairs = read_pairs(&associations)?;
    Ok(pairs
        .into_iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(extension))
        .map(|(_, command)| command.replace("{}", &super::files::quote(&path))))
}

/// The `key<TAB>value` lines of the file at `path`; none when it is missing.
/// Only a small regular file is read: a FIFO in its place froze the event
/// loop inside `open`.
fn read_pairs(path: &Path) -> io::Result<Vec<(String, String)>> {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let text = match read_small_file(path, &name) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    Ok(text
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.split_once('\t'))
        .filter(|(label, command)| !label.trim().is_empty() && !command.trim().is_empty())
        .map(|(label, command)| (label.trim().to_owned(), command.trim().to_owned()))
        .collect())
}

#[cfg(test)]
mod tests;
