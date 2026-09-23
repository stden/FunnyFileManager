//! Editable user commands and extension associations.

use crate::{
    menu::{MenuCommand, SettingsPage},
    models::{App, ChoiceTarget, MenuChoice, Overlay},
};
use std::{fs, io, path::PathBuf};

fn config_path(app: &App, filename: &str) -> PathBuf {
    app.settings
        .persistence
        .file()
        .and_then(std::path::Path::parent)
        .map_or_else(
            || app.get_active_panel_ref().path.join(".ffm"),
            std::path::Path::to_path_buf,
        )
        .join(filename)
}

pub fn edit(app: &mut App, command: MenuCommand) -> io::Result<()> {
    let (filename, template) = match command {
        MenuCommand::UserMenu => ("user-menu.txt", "# One command per line: Label<TAB>command\n# Example: List files\tpwd\n"),
        MenuCommand::Associations => ("associations.txt", "# One association per line: extension<TAB>command\n# Use {} for the quoted selected filename.\n# Example: txt\tmy-editor {}\n"),
        _ => return Err(io::Error::other("Not an editable command file")),
    };
    let path = config_path(app, filename);
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("No configuration directory"))?;
    fs::create_dir_all(parent)?;
    if !path.exists() {
        fs::write(&path, template)?;
    }
    // The same loader as F4, so the editor knows what the disk held and a
    // save notices another program's (or another instance's) write.
    let bytes = fs::read(&path)?;
    let mut editor = crate::editor::editor_for(app, path, &bytes)?;
    // `Label<TAB>command` is the format: Tab types the separator even when
    // no line holding one is left in the file.
    editor.format.indent = crate::models::Indent::Tabs;
    app.set_overlay(Overlay::Editor(editor));
    Ok(())
}

pub fn open(app: &mut App) -> io::Result<()> {
    let mut rows = read_pairs(&config_path(app, "user-menu.txt"))?
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
    let pairs = read_pairs(&config_path(app, "associations.txt"))?;
    Ok(pairs
        .into_iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(extension))
        .map(|(_, command)| command.replace("{}", &super::files::quote(&path))))
}

fn read_pairs(path: &std::path::Path) -> io::Result<Vec<(String, String)>> {
    let text = match fs::read_to_string(path) {
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
