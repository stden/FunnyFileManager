//! Purpose: Stable modal names cover every overlay without exposing its private contents.
use crate::models::Overlay;

macro_rules! overlay_names {
    ($($(#[$attribute:meta])* $pattern:pat => $name:literal,)+) => {
        pub(super) const fn kind(overlay: &Overlay) -> &'static str {
            match overlay { $($(#[$attribute])* $pattern => $name,)+ }
        }
    };
}

overlay_names! {
    Overlay::None => "none",
    Overlay::TempCleanup(_) => "temp_cleanup",
    Overlay::TokenUsage(_) => "token_usage",
    Overlay::Help => "help",
    Overlay::Wsl(_) => "wsl",
    Overlay::PathEditor(_) => "path_editor",
    Overlay::LspEditor(_) => "language_servers",
    Overlay::Viewer(_) => "viewer",
    Overlay::Editor(_) => "editor",
    Overlay::DriveSelection { .. } => "drive_selection",
    Overlay::NewFileInput(_) => "new_file_input",
    Overlay::CommandLine(_) => "command_line",
    Overlay::DeleteConfirm(_) => "delete_confirm",
    Overlay::OperationFailed(_) => "operation_failed",
    Overlay::ExistingFile(_) => "existing_file",
    Overlay::QuitConfirm(_) => "quit_confirm",
    Overlay::NamePrompt(_) => "name_prompt",
    Overlay::Information(_) => "information",
    Overlay::Menu(_) => "menu",
    Overlay::MenuDialog(_) => "menu_dialog",
    Overlay::MaskPrompt(_) => "mask_prompt",
    Overlay::CopyPrompt(_) => "copy_prompt",
    Overlay::WipeConfirm(_) => "wipe_confirm",
    Overlay::ArchivePrompt(_) => "archive_prompt",
    Overlay::ExtractPrompt(_) => "extract_prompt",
    Overlay::ArchiveCommands(_) => "archive_commands",
    Overlay::Attributes(_) => "attributes",
    Overlay::ApplyCommand(_) => "apply_command",
    Overlay::Describe(_) => "describe",
    Overlay::FindFile(_) => "find_file",
    Overlay::CommandHistory(_) => "command_history",
    Overlay::FindFolder(_) => "find_folder",
    Overlay::ViewHistory(_) => "view_history",
    Overlay::FolderHistory(_) => "folder_history",
    Overlay::UserMenu(_) => "user_menu",
    Overlay::Associations(_) => "associations",
    Overlay::FolderShortcuts(_) => "folder_shortcuts",
    Overlay::Filter(_) => "filter",
    Overlay::Screens(_) => "screens",
    Overlay::Tasks(_) => "tasks",
    Overlay::Hotplug(_) => "hotplug",
    Overlay::SpaceAnalysis(_) => "space_analysis",
    Overlay::Things(_) => "things",
    Overlay::McpInstall(_) => "mcp_install",
    Overlay::SortModes(_) => "sort_modes",
    Overlay::Settings(_) => "settings",
    Overlay::SessionEdit(_) => "session_edit",
    Overlay::Remote(_) => "remote",
    Overlay::PluginCommands(_) => "plugin_commands",
    Overlay::InputPrompt(_) => "input_prompt",
    Overlay::Question(_) => "question",
    Overlay::DiskCleanup(_) => "disk_cleanup",
    #[cfg(feature = "keepass")]
    Overlay::Kdbx(_) => "vault",
}
