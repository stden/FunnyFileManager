//! Purpose: The menu's own small dialogs — a list of choices to pick from and
//! a one-line prompt — the history and shortcut lists built on them, and what
//! picking a choice does.

use std::io;

use super::execute;
use crate::{
    lineedit::LineEditor,
    menu::MenuCommand,
    models::{App, ChoiceTarget, MenuChoice, MenuDialog, Overlay},
};

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

pub(super) fn prompt(app: &mut App, command: MenuCommand, title: &str, caption: &str, value: &str) {
    app.set_overlay(Overlay::MenuDialog(Box::new(MenuDialog::Prompt {
        command,
        title: title.to_owned(),
        caption: caption.to_owned(),
        input: LineEditor::new(value),
    })));
}

pub(super) fn history(app: &mut App) {
    let rows = (0..app.settings.number(crate::menu::SettingsPage::CommandLine))
        .map_while(|offset| app.history.recall(crate::history::RingIndex::new(offset)))
        .map(|text| MenuChoice {
            label: text.to_owned(),
            target: ChoiceTarget::Command(text.to_owned()),
        })
        .collect();
    choices(app, "Command history", rows);
}

pub(super) fn shortcuts(app: &mut App) {
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

pub(super) fn choices(app: &mut App, title: &str, rows: Vec<MenuChoice>) {
    app.set_overlay(Overlay::MenuDialog(Box::new(MenuDialog::Choices {
        title: title.to_owned(),
        rows,
        selected: 0,
    })));
}
