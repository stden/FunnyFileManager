//! Modal data for F9 prompts and selectable results.

use std::path::PathBuf;

use crate::{
    lineedit::LineEditor,
    menu::{MenuCommand, SettingsPage},
};

/// What the menu's one-line prompt asks for, and so what its answer does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuPrompt {
    /// The active panel's file mask (Commands → File panel filter).
    Filter,
    /// A folder mask to search for (Commands → Find folder).
    FindFolder,
    /// One Options page's value.
    Settings(SettingsPage),
}

#[derive(Debug, Clone)]
pub enum MenuDialog {
    Prompt {
        purpose: MenuPrompt,
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

/// A choice reads as its label, which is all a menu draws of it.
impl AsRef<str> for MenuChoice {
    fn as_ref(&self) -> &str {
        &self.label
    }
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
