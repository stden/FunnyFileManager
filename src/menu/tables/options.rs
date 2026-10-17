//! Purpose: the Options drop-down — one row per settings page, then Save setup.

use crate::menu::{Entry, MenuCommand, SettingsPage};

/// Editable settings and explicit persistence through Save setup.
pub(in crate::menu) const OPTIONS: [Entry; 27] = [
    Entry::command(
        "S&ystem settings",
        "",
        MenuCommand::Settings(SettingsPage::System),
    ),
    Entry::command(
        "&Panel settings",
        "",
        MenuCommand::Settings(SettingsPage::Panel),
    ),
    Entry::command(
        "&Tree settings",
        "",
        MenuCommand::Settings(SettingsPage::Tree),
    ),
    Entry::command(
        "&Interface settings",
        "",
        MenuCommand::Settings(SettingsPage::Interface),
    ),
    Entry::command(
        "Lan&guages",
        "",
        MenuCommand::Settings(SettingsPage::Language),
    ),
    Entry::command(
        "Pl&ugins configuration",
        "",
        MenuCommand::Settings(SettingsPage::Plugins),
    ),
    Entry::command(
        "Plugins manage&r settings",
        "",
        MenuCommand::Settings(SettingsPage::PluginManager),
    ),
    Entry::command(
        "Di&alog settings",
        "",
        MenuCommand::Settings(SettingsPage::Dialog),
    ),
    Entry::command(
        "Menu settings",
        "",
        MenuCommand::Settings(SettingsPage::Menu),
    ),
    Entry::command(
        "&Command line settings",
        "",
        MenuCommand::Settings(SettingsPage::CommandLine),
    ),
    Entry::command(
        "AutoComplete settings",
        "",
        MenuCommand::Settings(SettingsPage::AutoComplete),
    ),
    Entry::command(
        "Inf&oPanel settings",
        "",
        MenuCommand::Settings(SettingsPage::Information),
    ),
    Entry::command(
        "Groups of file mas&ks",
        "",
        MenuCommand::Settings(SettingsPage::Masks),
    ),
    Entry::Separator,
    Entry::command(
        "Co&nfirmations",
        "",
        MenuCommand::Settings(SettingsPage::Confirmations),
    ),
    Entry::command(
        "File panel &modes",
        "",
        MenuCommand::Settings(SettingsPage::PanelModes),
    ),
    Entry::command(
        "File &descriptions",
        "",
        MenuCommand::Settings(SettingsPage::Descriptions),
    ),
    Entry::command(
        "&Folder description files",
        "",
        MenuCommand::Settings(SettingsPage::FolderDescriptions),
    ),
    Entry::Separator,
    Entry::command(
        "&Viewer settings",
        "",
        MenuCommand::Settings(SettingsPage::Viewer),
    ),
    Entry::command(
        "&Editor settings",
        "",
        MenuCommand::Settings(SettingsPage::Editor),
    ),
    Entry::command(
        "Code pages",
        "",
        MenuCommand::Settings(SettingsPage::CodePages),
    ),
    Entry::Separator,
    Entry::command("Co&lors", "", MenuCommand::Settings(SettingsPage::Colors)),
    Entry::command(
        "Files &highlighting and sort groups",
        "",
        MenuCommand::Settings(SettingsPage::Highlighting),
    ),
    Entry::Separator,
    Entry::command("&Save setup", "Shift+F9", MenuCommand::SaveSetup),
];
