//! Purpose: the Options drop-down — one row per settings page, then Save setup.

use crate::action::Action;
use crate::menu::{Entry, MenuCommand, SettingsPage};

/// Editable settings and explicit persistence through Save setup.
pub(in crate::menu) const OPTIONS: [Entry; 27] = [
    Entry::item(
        "S&ystem settings",
        "",
        Action::Menu(MenuCommand::Settings(SettingsPage::System)),
    ),
    Entry::item(
        "&Panel settings",
        "",
        Action::Menu(MenuCommand::Settings(SettingsPage::Panel)),
    ),
    Entry::item(
        "&Tree settings",
        "",
        Action::Menu(MenuCommand::Settings(SettingsPage::Tree)),
    ),
    Entry::item(
        "&Interface settings",
        "",
        Action::Menu(MenuCommand::Settings(SettingsPage::Interface)),
    ),
    Entry::item(
        "Lan&guages",
        "",
        Action::Menu(MenuCommand::Settings(SettingsPage::Language)),
    ),
    Entry::item(
        "Pl&ugins configuration",
        "",
        Action::Menu(MenuCommand::Settings(SettingsPage::Plugins)),
    ),
    Entry::item(
        "Plugins manage&r settings",
        "",
        Action::Menu(MenuCommand::Settings(SettingsPage::PluginManager)),
    ),
    Entry::item(
        "Di&alog settings",
        "",
        Action::Menu(MenuCommand::Settings(SettingsPage::Dialog)),
    ),
    Entry::item(
        "Menu settings",
        "",
        Action::Menu(MenuCommand::Settings(SettingsPage::Menu)),
    ),
    Entry::item(
        "&Command line settings",
        "",
        Action::Menu(MenuCommand::Settings(SettingsPage::CommandLine)),
    ),
    Entry::item(
        "AutoComplete settings",
        "",
        Action::Menu(MenuCommand::Settings(SettingsPage::AutoComplete)),
    ),
    Entry::item(
        "Inf&oPanel settings",
        "",
        Action::Menu(MenuCommand::Settings(SettingsPage::Information)),
    ),
    Entry::item(
        "Groups of file mas&ks",
        "",
        Action::Menu(MenuCommand::Settings(SettingsPage::Masks)),
    ),
    Entry::Separator,
    Entry::item(
        "Co&nfirmations",
        "",
        Action::Menu(MenuCommand::Settings(SettingsPage::Confirmations)),
    ),
    Entry::item(
        "File panel &modes",
        "",
        Action::Menu(MenuCommand::Settings(SettingsPage::PanelModes)),
    ),
    Entry::item(
        "File &descriptions",
        "",
        Action::Menu(MenuCommand::Settings(SettingsPage::Descriptions)),
    ),
    Entry::item(
        "&Folder description files",
        "",
        Action::Menu(MenuCommand::Settings(SettingsPage::FolderDescriptions)),
    ),
    Entry::Separator,
    Entry::item(
        "&Viewer settings",
        "",
        Action::Menu(MenuCommand::Settings(SettingsPage::Viewer)),
    ),
    Entry::item(
        "&Editor settings",
        "",
        Action::Menu(MenuCommand::Settings(SettingsPage::Editor)),
    ),
    Entry::item(
        "Code pages",
        "",
        Action::Menu(MenuCommand::Settings(SettingsPage::CodePages)),
    ),
    Entry::Separator,
    Entry::item(
        "Co&lors",
        "",
        Action::Menu(MenuCommand::Settings(SettingsPage::Colors)),
    ),
    Entry::item(
        "Files &highlighting and sort groups",
        "",
        Action::Menu(MenuCommand::Settings(SettingsPage::Highlighting)),
    ),
    Entry::Separator,
    Entry::item(
        "&Save setup",
        "Shift+F9",
        Action::Menu(MenuCommand::SaveSetup),
    ),
];
