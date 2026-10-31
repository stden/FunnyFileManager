//! Handles modal overlay input and routes each dialog to its owner.

use crate::models::{App, Overlay};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

mod disk_cleanup;
mod editor_search;
mod lsp_editor;
mod menu_dialog;
mod path_env;
mod remote_dialog;
mod token_usage;

use editor_search::{process_search_prompt, repeat_search};

mod paste;
pub(crate) use paste::paste_text;

/// Handle keyboard input for popup overlays (help, viewer, editor, dialogs).
///
/// Overlays are modal windows that appear on top of the panels.
/// While an overlay is open, keys go here instead of the main handler.
/// Returns true if the key was consumed by an overlay.
pub fn process_popup_input(app: &mut App, key: KeyEvent) -> bool {
    let code = key.code;
    let modifiers = key.modifiers;
    match &app.overlay {
        Overlay::None => false,
        Overlay::MenuDialog(_) => menu_dialog::process(app, code, modifiers),
        Overlay::Wsl(_) => wsl::process(app, code, modifiers),
        Overlay::PathEditor(_) => path_env::process(app, code, modifiers),
        Overlay::LspEditor(_) => lsp_editor::process(app, code, modifiers),
        Overlay::Viewer(_) => viewer::process(app, code, modifiers),
        Overlay::Editor(_) => process_editor_input(app, code, modifiers),
        Overlay::Help => confirmation::help(app, code),
        Overlay::Information(_) => confirmation::information(app, code),
        Overlay::QuitConfirm(_) => confirmation::quit(app, code),
        Overlay::NamePrompt(_) => name_prompt::process(app, code, modifiers),
        Overlay::MaskPrompt(_) => mask_prompt::process(app, code, modifiers),
        Overlay::CopyPrompt(_) => copy_prompt::process(app, code, modifiers),
        Overlay::Menu(_) => menu::process(app, code, modifiers),
        Overlay::CommandLine(_) => command_line::process(app, key),
        Overlay::DriveSelection { .. } => drives::process(app, code),
        Overlay::DeleteConfirm(_) => confirmation::delete(app, code),
        Overlay::OperationFailed(_) => confirmation::operation_failed(app, code),
        Overlay::ExistingFile(_) => confirmation::existing_file(app, code),
        Overlay::NewFileInput(_) => new_file::process(app, code, modifiers),
        #[cfg(feature = "keepass")]
        Overlay::Kdbx(_) => kdbx::process_kdbx_input(app, code, modifiers),
        Overlay::DiskCleanup(_) => disk_cleanup::process(app, code, modifiers),
        _ => process_menu_dialog(app, code, modifiers),
    }
}

/// The Files-menu dialogs the completed F9 menu brought, one owner each.
///
/// Every owner receives the modifiers, whether it reads them yet or not: the
/// History list wants `Ctrl+Enter` and the view history `Shift+Enter`, and an
/// owner may not edit this router to ask for them later.
///
/// A second and third function only to keep each under the line limit; the
/// split follows the menu headings.
fn process_menu_dialog(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    match &app.overlay {
        Overlay::TokenUsage(_) => token_usage::process(app, code, modifiers),
        Overlay::WipeConfirm(_) => wipe::process(app, code, modifiers),
        Overlay::ArchivePrompt(_) => archive_prompt::process(app, code, modifiers),
        Overlay::ExtractPrompt(_) => extract_prompt::process(app, code, modifiers),
        Overlay::ArchiveCommands(_) => archive_commands::process(app, code, modifiers),
        Overlay::Attributes(_) => attributes::process(app, code, modifiers),
        Overlay::ApplyCommand(_) => apply::process(app, code, modifiers),
        Overlay::Describe(_) => describe::process(app, code, modifiers),
        _ => process_commands_dialog(app, code, modifiers),
    }
}

/// The Commands- and Options-menu dialogs. Every variant is named in one of
/// the three routers; the fallback here is only for the arms above.
fn process_commands_dialog(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    match &app.overlay {
        Overlay::FindFile(_) => findfile::process(app, code, modifiers),
        Overlay::CommandHistory(_) => history_list::process(app, code, modifiers),
        Overlay::FindFolder(_) => findfolder::process(app, code, modifiers),
        Overlay::ViewHistory(_) => view_history::process(app, code, modifiers),
        Overlay::FolderHistory(_) => folder_history::process(app, code, modifiers),
        Overlay::UserMenu(_) => usermenu::process(app, code, modifiers),
        Overlay::Associations(_) => associations::process(app, code, modifiers),
        Overlay::FolderShortcuts(_) => shortcuts::process(app, code, modifiers),
        Overlay::Filter(_) => filter::process(app, code, modifiers),
        Overlay::Screens(_) => screens::process(app, code, modifiers),
        Overlay::Tasks(_) => tasks::process(app, code, modifiers),
        Overlay::Hotplug(_) => hotplug::process(app, code, modifiers),
        Overlay::McpInstall(_) => mcp_install::process(app, code, modifiers),
        Overlay::SortModes(_) => sortmenu::process(app, code, modifiers),
        Overlay::Settings(_) => settings::process(app, code, modifiers),
        Overlay::SessionEdit(_) => session_edit::process(app, code, modifiers),
        Overlay::Remote(_) => remote_dialog::process(app, code, modifiers),
        Overlay::PluginCommands(_) => plugin_commands::process(app, code, modifiers),
        Overlay::InputPrompt(_) => input_prompt::process(app, code, modifiers),
        Overlay::Question(_) => question::process(app, code, modifiers),
        _ => false,
    }
}

/// Lines jumped by PgUp/PgDn in the built-in editor.
const EDITOR_PAGE: usize = 20;

mod apply;
mod archive_commands;
mod archive_prompt;
mod associations;
mod attributes;
mod command_line;
mod confirmation;
mod copy_prompt;
mod describe;
mod drives;
mod editor;
mod extract_prompt;
mod filter;
mod findfile;
mod findfolder;
mod folder_history;
mod history_list;
mod hotplug;
mod input_prompt;
mod mask_prompt;
mod mcp_install;
mod menu;
mod name_prompt;
mod netbox;
mod new_file;
mod plugin_commands;
mod question;
pub(crate) mod quickview;
pub(crate) mod remote;
mod screens;
mod session_edit;
pub(crate) mod sessions;
mod settings;
mod shortcuts;
mod sortmenu;
mod stub;
mod tasks;
pub(crate) mod text_field;
pub(crate) mod tree;
mod usermenu;
mod view_history;
mod viewer;
mod wheel;
mod wipe;
pub(crate) mod wsl;
use editor::process_editor_input;
pub(crate) use wheel::wheel;
#[cfg(feature = "keepass")]
pub(crate) mod kdbx;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_command_line_completion;
#[cfg(test)]
mod tests_command_line_editing;
#[cfg(test)]
mod tests_console_and_editor_modes;
#[cfg(test)]
mod tests_dialog_keys;
#[cfg(test)]
mod tests_editor_search;
#[cfg(test)]
mod tests_paste_targets;
#[cfg(test)]
mod tests_text_fields;
#[cfg(test)]
mod tests_viewer_keys;
