//! Modal overlay rendering: help, viewer, built-in editor, dialogs.

mod copy_prompt;
mod editor;
mod editor_dialogs;
mod information;
mod name_prompt;
mod panel_dialogs;

use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::dialog;
use super::palette;
use super::{centered_rect, far_block};
use crate::models::{App, Overlay};
use copy_prompt::render_copy_prompt;
use editor::render_editor;
use information::render_information;
use name_prompt::render_name_prompt;
pub(super) use panel_dialogs::render_mask_prompt;
use panel_dialogs::{render_delete_confirm, render_new_file_input, render_quit_confirm};

#[cfg(test)]
mod tests;

pub fn render_overlays(f: &mut Frame<'_>, app: &mut App) {
    if !app.overlay.is_active() {
        // No dialog this frame, so last frame's rectangles describe nothing.
        // Leaving them would send a click on the panels into a closed window.
        app.dialog_mouse.clear();
    }
    match &mut app.overlay {
        // Nothing to draw on top. The command line is part of the main
        // layout, and the menu hangs from the top row rather than floating in
        // the middle of the screen — both are drawn by whoever owns that row.
        Overlay::None | Overlay::CommandLine(_) | Overlay::Menu(_) => {}
        Overlay::Wsl(dialog) => super::wsl::render(f, dialog),
        Overlay::PathEditor(dialog) => super::path_env::render(f, dialog),
        Overlay::LspEditor(dialog) => super::lsp_editor::render(f, &mut app.dialog_mouse, dialog),
        Overlay::MenuDialog(dialog) => super::menu_dialog::render(f, dialog, &mut app.dialog_mouse),
        Overlay::Help => render_help(f),
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
        Overlay::Information(stats) => render_information(f, &mut app.dialog_mouse, stats),
        Overlay::CopyPrompt(prompt) => render_copy_prompt(f, &mut app.dialog_mouse, prompt),
        Overlay::QuitConfirm(answer) => render_quit_confirm(f, &mut app.dialog_mouse, *answer),
        Overlay::DeleteConfirm(files) => render_delete_confirm(f, &mut app.dialog_mouse, files),
        Overlay::NewFileInput(input) => render_new_file_input(f, &mut app.dialog_mouse, input),
        #[cfg(feature = "keepass")]
        Overlay::Kdbx(overlay) => super::kdbx::render_kdbx(f, &overlay.0, &mut app.dialog_mouse),
        other => render_menu_dialog(f, other, &mut app.dialog_mouse),
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
) {
    match overlay {
        Overlay::WipeConfirm(dialog) => super::wipe::render(f, tracker, dialog),
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
        Overlay::FindFile(dialog) => super::findfile::render(f, tracker, dialog),
        Overlay::CommandHistory(dialog) => super::history_list::render(f, tracker, dialog),
        Overlay::FindFolder(dialog) => super::findfolder::render(f, tracker, dialog),
        Overlay::ViewHistory(dialog) => super::view_history::render(f, tracker, dialog),
        Overlay::FolderHistory(dialog) => super::folder_history::render(f, tracker, dialog),
        Overlay::UserMenu(dialog) => super::usermenu::render(f, tracker, dialog),
        Overlay::Associations(dialog) => super::associations::render(f, tracker, dialog),
        Overlay::FolderShortcuts(dialog) => super::shortcuts::render(f, tracker, dialog),
        Overlay::Filter(dialog) => super::filter::render(f, tracker, dialog),
        Overlay::Screens(dialog) => super::screens::render(f, tracker, dialog),
        Overlay::Hotplug(dialog) => super::hotplug::render(f, tracker, dialog),
        Overlay::McpInstall(dialog) => super::mcp_install::render(f, tracker, dialog),
        Overlay::SortModes(dialog) => super::sortmenu::render(f, tracker, dialog),
        Overlay::Settings(dialog) => super::settings::render(f, tracker, dialog),
        Overlay::DiskCleanup(dialog) => super::disk_cleanup::render(f, tracker, dialog),
        Overlay::SessionEdit(dialog) => super::session_edit::render(f, tracker, dialog),
        Overlay::PluginCommands(dialog) => super::plugin_commands::render(f, tracker, dialog),
        Overlay::InputPrompt(prompt) => super::prompts::render_input(f, tracker, prompt),
        Overlay::Question(question) => super::prompts::render_question(f, tracker, question),
        _ => {}
    }
}

const HELP_LINES: &[&str] = &[
    "Far Manager style navigation:",
    "F1: Toggle this help window",
    "F2: Open the user command menu",
    "F3: View selected file (read-only)",
    "F4: Edit selected file (built-in editor, F2 saves)",
    "  in the editor: F7 search, Shift+F7 search again",
    "Alt+F4: Edit selected file with $EDITOR or VS Code",
    "F5: Copy selected entry to other panel",
    "F6: Move/Rename selected entry to other panel",
    "F7: Create a new directory in active panel",
    "F8: Delete selected entry",
    "F9: Open the menu bar",
    "F10: Quit (or type `quit`)",
    "F11: Plugin commands (MCP status, NetBox sessions and imports)",
    "F12: Switch panels / console screen",
    "Ctrl+W: Task list (processes; F1 inside lists its keys)",
    "Ctrl+O: Show the console screen (press again for the panels)",
    "Tab: switch active panel",
    "PgUp/PgDn: move the selection one screen up/down",
    ":  - command line (Tab completes verbs, args & cd paths)",
    ":mcp - MCP server status & installation",
    ":winmsg, :winperf - Windows warnings and errors, performance summary",
];

fn render_help(f: &mut Frame<'_>) {
    let area = centered_rect(80, 80, f.area());
    dialog::render_shadow(f, area, f.area());

    let content = HELP_LINES
        .iter()
        .copied()
        .map(|line| Line::from(Span::raw(line)))
        .collect::<Vec<Line<'_>>>();

    let style = Style::default().fg(palette::HELP_FG).bg(palette::HELP_BG);
    let block = far_block()
        .title(Line::from(" Help ").centered())
        .border_style(style)
        .style(style);

    let paragraph = Paragraph::new(content).block(block).style(style);

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}
