//! Purpose: the `panels` rows no key runs by default — the F9 rows Far
//! gives no key and this program's own, then the pages of the Options
//! menu. Each is an action like any other and can be given a key.

use crate::action::Action;
use crate::keymap::Origin::{FarNoKey, Ffm};
use crate::keymap::{row, Group};
use crate::menu::{MenuCommand, SettingsPage as Page};
use crate::path_env::Scope;

use Action::Menu;
use MenuCommand::Settings;

pub(super) const KEYLESS: Group<Action> = Group {
    title: "F9 commands without a default key",
    rows: &[
        row(
            "system_temp_cleanup",
            Menu(MenuCommand::SystemTempCleanup),
            &[],
        )
        .note(Ffm, "Clean old temporary files"),
        row("token_usage", Menu(MenuCommand::TokenUsage), &[])
            .note(Ffm, "Provider token usage reports"),
        row("things_catalog", Menu(MenuCommand::ThingsCatalog), &[])
            .note(Ffm, "Import Things catalog machines as sessions"),
        row("compare_folders", Menu(MenuCommand::Compare), &[]).note(FarNoKey, "Compare folders"),
        row("edit_user_menu", Menu(MenuCommand::UserMenu), &[])
            .note(FarNoKey, "Edit the user menu"),
        row("file_associations", Menu(MenuCommand::Associations), &[])
            .note(FarNoKey, "File associations"),
        row("folder_shortcuts", Menu(MenuCommand::Shortcuts), &[])
            .note(FarNoKey, "Folder shortcuts"),
        row("hotplug_devices", Menu(MenuCommand::Devices), &[])
            .note(FarNoKey, "Hotplug devices list"),
        row("install_mcp_server", Menu(MenuCommand::McpInstall), &[])
            .note(Ffm, "Install the MCP server for AI clients"),
        row("language_servers", Menu(MenuCommand::LanguageServers), &[])
            .note(Ffm, "Language servers of the editor"),
        row("update_packages", Menu(MenuCommand::UpdatePackages), &[])
            .note(Ffm, "Update the system's packages"),
        row("action_statistics", Menu(MenuCommand::Performance), &[])
            .note(Ffm, "How long each action took"),
        row("edit_path_user", Menu(MenuCommand::Path(Scope::User)), &[])
            .note(Ffm, "Edit PATH of the current user"),
        row(
            "edit_path_system",
            Menu(MenuCommand::Path(Scope::System)),
            &[],
        )
        .note(Ffm, "Edit PATH of all users"),
        row("git_pull_all", Menu(MenuCommand::GitPullAll), &[])
            .note(Ffm, "Git pull for all repositories"),
        row("space_analysis", Menu(MenuCommand::SpaceAnalysis), &[])
            .note(Ffm, "Space analysis of the active folder"),
    ],
};

/// A page of the Options menu, under the title its dialog shows.
const fn page(name: &'static str, page: Page) -> crate::keymap::Row<Action> {
    row(name, Menu(Settings(page)), &[]).note(FarNoKey, page.title())
}

pub(super) const OPTIONS: Group<Action> = Group {
    title: "Options pages without a default key",
    rows: &[
        page("settings_system", Page::System),
        page("settings_panel", Page::Panel),
        page("settings_tree", Page::Tree),
        page("settings_interface", Page::Interface),
        page("settings_language", Page::Language),
        page("settings_plugins", Page::Plugins),
        page("settings_plugin_manager", Page::PluginManager),
        page("settings_dialog", Page::Dialog),
        page("settings_menu", Page::Menu),
        page("settings_command_line", Page::CommandLine),
        page("settings_auto_complete", Page::AutoComplete),
        page("settings_information", Page::Information),
        page("settings_masks", Page::Masks),
        page("settings_confirmations", Page::Confirmations),
        page("settings_panel_modes", Page::PanelModes),
        page("settings_descriptions", Page::Descriptions),
        page("settings_folder_descriptions", Page::FolderDescriptions),
        page("settings_viewer", Page::Viewer),
        page("settings_editor", Page::Editor),
        page("settings_code_pages", Page::CodePages),
        page("settings_colors", Page::Colors),
        page("settings_highlighting", Page::Highlighting),
    ],
};

#[cfg(test)]
mod tests;
