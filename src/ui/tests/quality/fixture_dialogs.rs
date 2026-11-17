//! Representative editable forms, warnings and every static menu dialog.

use std::path::PathBuf;

use crate::{
    action::Action,
    menu::{Heading, Menu, MenuCommand, SettingsPage},
    models::{
        Confirm, FilterPrompt, HistoryList, NamePrompt, Overlay, ScreenList, SettingsDialog,
        ShortcutList,
    },
};

use super::{overlay, Case};

pub(super) fn cases() -> Vec<Case> {
    let mut cases = name_forms();
    cases.extend(confirmations());
    cases.extend(history_dialogs());
    cases.extend(list_dialogs());
    cases.extend(settings());
    cases.push(path_editor());
    cases.push(overlay(
        "filter",
        Overlay::Filter(FilterPrompt::default()),
        &["File panel filter", "Apply", "Cancel"],
    ));
    cases
}

fn name_forms() -> Vec<Case> {
    vec![
        overlay(
            "new-file",
            Overlay::NewFileInput("новый-界-e\u{301}.txt".into()),
            &["Create New File", "OK", "Cancel"],
        ),
        overlay(
            "rename",
            Overlay::NamePrompt(NamePrompt::rename("old.txt".into())),
            &["Rename", "OK", "Cancel"],
        ),
        overlay(
            "make-folder",
            Overlay::NamePrompt(NamePrompt::make_folder()),
            &["Make folder", "OK", "Cancel"],
        ),
    ]
}

fn confirmations() -> Vec<Case> {
    vec![
        overlay(
            "delete",
            Overlay::DeleteConfirm(vec![PathBuf::from("/ffm-quality/selected.txt")]),
            &["Delete", "Cancel"],
        ),
        overlay(
            "quit-yes",
            Overlay::QuitConfirm(Confirm::Yes),
            &["Quit", "Yes", "No"],
        ),
        overlay(
            "quit-no",
            Overlay::QuitConfirm(Confirm::No),
            &["Quit", "Yes", "No"],
        ),
    ]
}

fn history_dialogs() -> Vec<Case> {
    vec![
        coming(
            "history",
            Overlay::CommandHistory(HistoryList::default()),
            "History",
        ),
        coming(
            "find-folder",
            Overlay::FindFolder(Box::default()),
            "Find folder",
        ),
        coming(
            "view-history",
            Overlay::ViewHistory(HistoryList::default()),
            "File view history",
        ),
        coming(
            "folder-history",
            Overlay::FolderHistory(HistoryList::default()),
            "Folders history",
        ),
    ]
}

fn list_dialogs() -> Vec<Case> {
    vec![
        coming("user-menu", Overlay::UserMenu(Box::default()), "User menu"),
        coming(
            "associations",
            Overlay::Associations(Box::default()),
            "File associations",
        ),
        coming(
            "shortcuts",
            Overlay::FolderShortcuts(ShortcutList::default()),
            "Folder shortcuts",
        ),
        coming(
            "screens",
            Overlay::Screens(ScreenList::default()),
            "Screens",
        ),
        coming(
            "hotplug",
            Overlay::Hotplug(Box::default()),
            "Hotplug devices list",
        ),
    ]
}

fn coming(name: &'static str, state: Overlay, title: &'static str) -> Case {
    overlay(name, state, &[title, "Ok"])
}

fn settings() -> Vec<Case> {
    Menu::open(Heading::Options)
        .entries()
        .iter()
        .filter_map(|entry| match entry.action() {
            Some(Action::Menu(MenuCommand::Settings(page))) => Some(page),
            _ => None,
        })
        .map(setting)
        .collect()
}

fn setting(page: SettingsPage) -> Case {
    let mut case = overlay(
        page.title(),
        Overlay::Settings(Box::new(SettingsDialog::for_page(page))),
        &["Ok"],
    );
    if page == SettingsPage::Highlighting {
        // The 40-column dialog has 32 cells for its 34-cell title.
        case.required.push("Files highlighting");
        case.wide_required.push(page.title());
    } else {
        case.required.push(page.title());
    }
    case
}

fn path_editor() -> Case {
    use crate::path_env::{PathEditor, Scope, Snapshot, Validation};

    let mut editor = PathEditor::new(
        Scope::User,
        Snapshot {
            value: "/ffm-quality/missing".to_owned(),
            revision: String::new(),
            location: String::new(),
        },
    );
    editor
        .entries
        .first_mut()
        .expect("fixture entry")
        .validation = Validation::Missing;
    overlay(
        "path-error",
        Overlay::PathEditor(Box::new(editor)),
        &["PATH", "MISSING"],
    )
}
