//! Modal data for F9 prompts and selectable results.

use std::path::PathBuf;

use crate::{lineedit::LineEditor, menu::MenuCommand};

#[derive(Debug, Clone)]
pub enum MenuDialog {
    Prompt {
        command: MenuCommand,
        title: String,
        caption: String,
        input: LineEditor,
    },
    Choices {
        title: String,
        rows: Vec<MenuChoice>,
        selected: usize,
    },
}

#[derive(Debug, Clone)]
pub struct MenuChoice {
    pub label: String,
    pub target: ChoiceTarget,
}

#[derive(Debug, Clone)]
pub enum ChoiceTarget {
    UpdatePackages(crate::packages::UpdatePlan),
    Directory(PathBuf),
    File(PathBuf),
    Command(String),
    Screen(super::ViewMode),
    Menu(MenuCommand),
    Close,
}
