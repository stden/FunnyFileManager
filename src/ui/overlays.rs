//! Modal overlay rendering: help, viewer, built-in editor, dialogs.

mod copy_prompt;
mod editor;
mod editor_dialogs;
mod existing_file;
mod help;
mod information;
mod name_prompt;
mod operation_failed;
mod panel_dialogs;

use ratatui::Frame;

use crate::models::{App, Overlay};
use copy_prompt::render_copy_prompt;
use editor::render_editor;
use help::render_help;
use information::render_information;
use name_prompt::render_name_prompt;
use operation_failed::render_operation_failed;
pub(super) use panel_dialogs::render_mask_prompt;
use panel_dialogs::{
    render_delete_confirm, render_new_file_input, render_quit_confirm, single_target_noun,
};

#[cfg(test)]
mod tests;

pub fn render_overlays(f: &mut Frame<'_>, app: &mut App) {
    clear_closed_dialog(app);
    let noun = single_target_noun(app);
    match &mut app.overlay {
        // Nothing to draw on top. The command line is part of the main
        // layout, and the menu hangs from the top row rather than floating in
        // the middle of the screen — both are drawn by whoever owns that row.
        Overlay::None | Overlay::CommandLine(_) | Overlay::Menu(_) => {}
        Overlay::Wsl(dialog) => super::wsl::render(f, dialog),
        Overlay::PathEditor(dialog) => super::path_env::render(f, dialog),
        Overlay::LspEditor(dialog) => super::lsp_editor::render(f, &mut app.dialog_mouse, dialog),
        Overlay::MenuDialog(dialog) => super::menu_dialog::render(f, dialog, &mut app.dialog_mouse),
        Overlay::Help => render_help(f, app.help_top),
        Overlay::Filter(dialog) => {
            super::filter::render(f, &mut app.dialog_mouse, dialog, &app.filter_input);
        }
        Overlay::Viewer(viewer) => super::viewer::render_viewer(f, viewer),
        Overlay::Editor(editor) => render_editor(f, editor, &mut app.dialog_mouse),
        Overlay::Tasks(list) => super::tasks::render(f, &mut app.dialog_mouse, list),
        Overlay::DriveSelection {
            side: target_panel,
            selected_index,
            rows,
        } => super::drives::render(f, *target_panel, rows, *selected_index),
        Overlay::NamePrompt(prompt) => render_name_prompt(f, &mut app.dialog_mouse, prompt),
        Overlay::MaskPrompt(prompt) => render_mask_prompt(f, &mut app.dialog_mouse, prompt),
        Overlay::CopyPrompt(prompt) => render_copy_prompt(f, &mut app.dialog_mouse, prompt),
        Overlay::QuitConfirm(answer) => render_quit_confirm(f, &mut app.dialog_mouse, *answer),
        Overlay::NewFileInput(input) => render_new_file_input(f, &mut app.dialog_mouse, input),
        #[cfg(feature = "keepass")]
        Overlay::Kdbx(overlay) => super::kdbx::render_kdbx(f, &overlay.0, &mut app.dialog_mouse),
        other => render_menu_dialog(f, other, &mut app.dialog_mouse, noun),
    }
}

/// Forget stale hit regions when no overlay owns the screen.
fn clear_closed_dialog(app: &mut App) {
    if !app.overlay.is_active() {
        app.dialog_mouse.clear();
    }
}

/// The Files-menu dialogs the completed F9 menu brought, one owner each.
///
/// A second and third function only to keep each under the line limit; the
/// split follows the menu headings.
fn render_menu_dialog(
    f: &mut Frame<'_>,
    overlay: &mut Overlay,
    tracker: &mut crate::mouse::DialogMouse,
    noun: &str,
) {
    match overlay {
        Overlay::TokenUsage(panel) => super::viewer::render_viewer(f, &panel.viewer),
        Overlay::Information(stats) => render_information(f, tracker, stats),
        Overlay::DeleteConfirm(files) => render_delete_confirm(f, tracker, files, noun),
        Overlay::OperationFailed(failure) => render_operation_failed(f, tracker, failure),
        Overlay::ExistingFile(question) => existing_file::render(f, tracker, question),
        Overlay::WipeConfirm(dialog) => super::wipe::render(f, tracker, dialog, noun),
        Overlay::ArchivePrompt(dialog) => super::archive_prompt::render(f, tracker, dialog),
        Overlay::ExtractPrompt(dialog) => super::extract_prompt::render(f, tracker, dialog),
        Overlay::ArchiveCommands(dialog) => super::archive_commands::render(f, tracker, dialog),
        Overlay::Attributes(dialog) => super::attributes::render(f, tracker, dialog),
        Overlay::ApplyCommand(dialog) => super::apply::render(f, tracker, dialog),
        Overlay::Describe(dialog) => super::describe::render(f, tracker, dialog),
        other => render_commands_dialog(f, other, tracker),
    }
}

/// The Commands- and Options-menu dialogs. Every variant is drawn by one of
/// the three routers; the fallback here is only for the arms above.
fn render_commands_dialog(
    f: &mut Frame<'_>,
    overlay: &mut Overlay,
    tracker: &mut crate::mouse::DialogMouse,
) {
    match overlay {
        Overlay::TempCleanup(dialog) => super::system_temp_cleanup::render(f, tracker, dialog),
        Overlay::FindFile(dialog) => super::findfile::render(f, tracker, dialog),
        Overlay::CommandHistory(dialog) => super::history_list::render(f, tracker, dialog),
        Overlay::FindFolder(dialog) => super::findfolder::render(f, tracker, dialog),
        Overlay::ViewHistory(dialog) => super::view_history::render(f, tracker, dialog),
        Overlay::FolderHistory(dialog) => super::folder_history::render(f, tracker, dialog),
        Overlay::UserMenu(dialog) => super::usermenu::render(f, tracker, dialog),
        Overlay::Associations(dialog) => super::associations::render(f, tracker, dialog),
        Overlay::FolderShortcuts(dialog) => super::shortcuts::render(f, tracker, dialog),
        Overlay::Screens(dialog) => super::screens::render(f, tracker, dialog),
        Overlay::Hotplug(dialog) => super::hotplug::render(f, tracker, dialog),
        Overlay::McpInstall(dialog) => super::mcp_install::render(f, tracker, dialog),
        Overlay::SpaceAnalysis(dialog) => super::space_analysis::render(f, tracker, dialog),
        Overlay::Things(dialog) => super::things::render(f, tracker, dialog),
        Overlay::SortModes(dialog) => super::sortmenu::render(f, tracker, dialog),
        Overlay::Settings(dialog) => super::settings::render(f, tracker, dialog),
        Overlay::DiskCleanup(dialog) => super::disk_cleanup::render(f, tracker, dialog),
        Overlay::SessionEdit(dialog) => super::session_edit::render(f, tracker, dialog),
        Overlay::Remote(dialog) => super::remote_dialog::render(f, tracker, dialog),
        Overlay::PluginCommands(dialog) => super::plugin_commands::render(f, tracker, dialog),
        Overlay::InputPrompt(prompt) => super::prompts::render_input(f, tracker, prompt),
        Overlay::Question(question) => super::prompts::render_question(f, tracker, question),
        _ => {}
    }
}

#[cfg(test)]
mod tests_commands;
