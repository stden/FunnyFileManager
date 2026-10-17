//! Purpose: apply the resolved [`Action`] families to the panels view.

use std::io::Write;

use ratatui::{backend::Backend, Terminal};

use crate::action::Action;
use crate::editor::{handle_edit, handle_edit_builtin};
use crate::models::{self, App, CommandLine, Lifecycle, NamePrompt, Overlay};
use crate::ops::{handle_delete_confirm, handle_view};
use crate::{copy, ops, status};

use super::navigation::{
    apply_navigation, quick_search, select_panel_or_column, shorten_quick_search,
};
use super::prompts::{
    open_command_line_with, open_drive_menu, open_mask_prompt, open_menu, open_rename_prompt,
    open_transfer_prompt, toggle_help,
};
use super::report;

pub(super) fn apply_panel_action<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    action: Action,
) {
    match action {
        Action::Menu(command) => ops::menu::execute(app, command),
        Action::PanelView(mode) => report(app, |app| {
            ops::panel_details::set_view(app.get_active_panel(), mode)
        }),
        Action::Navigate(command) => apply_navigation(terminal, app, command),
        Action::Selection(command) => apply_selection(app, command),
        Action::Dialog(command) => open_dialog(app, command),
        Action::Panel(command) => ops::menu::execute_panel(app, command),
        Action::File(command) => ops::menu::execute_file(app, command),
        Action::Tool(command) => report(app, |app| ops::menu::execute_tool(app, command)),
        plain => apply_plain_action(terminal, app, plain),
    }
}

/// The actions that are not grouped into a family: one effect each.
pub(super) fn apply_plain_action<B>(terminal: &mut Terminal<B>, app: &mut App, action: Action)
where
    B: Backend + Write,
{
    match action {
        Action::Menu(command) => ops::menu::execute(app, command),
        Action::PanelView(mode) => report(app, |app| {
            ops::panel_details::set_view(app.get_active_panel(), mode)
        }),
        Action::Quit => app.lifecycle = Lifecycle::Quitting,
        Action::SelectPanel(side) => select_panel_or_column(terminal, app, side),
        Action::DriveMenu(side) => open_drive_menu(app, side),
        Action::View => report(app, handle_view),
        Action::Edit => report(app, handle_edit_builtin),
        Action::EditExternal => report(app, |app| handle_edit(terminal, app)),
        Action::Copy => open_transfer_prompt(app, copy::Purpose::Copy),
        Action::Move => open_transfer_prompt(app, copy::Purpose::Move),
        Action::Rename => open_rename_prompt(app),
        Action::DeleteConfirm => report(app, handle_delete_confirm),
        Action::PluginCommands => ops::plugin_commands::open(app),
        Action::ToggleConsole => app.view = app.view.toggled(),
        Action::Refresh => refresh(app),
        Action::CommandLine => app.set_overlay(Overlay::CommandLine(CommandLine::default())),
        Action::CommandLineChar(ch) => open_command_line_with(app, ch),
        Action::QuickSearch(c) => quick_search(app, c),
        Action::QuickSearchBackspace => shorten_quick_search(app),
        Action::QuickSearchClear => app.quick_search.clear(),
        // The families are dispatched by `apply_action` before this is reached.
        Action::Navigate(_)
        | Action::Selection(_)
        | Action::Dialog(_)
        | Action::Panel(_)
        | Action::File(_)
        | Action::Tool(_) => {}
    }
}

fn apply_selection(app: &mut App, command: crate::action::SelectionCommand) {
    use crate::action::SelectionCommand;
    match command {
        SelectionCommand::SelectGroup => open_mask_prompt(app, models::MaskPurpose::Select),
        SelectionCommand::UnselectGroup => open_mask_prompt(app, models::MaskPurpose::Unselect),
        SelectionCommand::InvertSelection => {
            app.get_active_panel().invert_marks();
            report_selection(app);
        }
        SelectionCommand::RestoreSelection => {
            app.get_active_panel().restore_marks();
            report_selection(app);
        }
        SelectionCommand::ToggleMark => {
            app.get_active_panel().toggle_mark_selected();
            app.get_active_panel().next();
        }
    }
}

fn report_selection(app: &mut App) {
    let marked = app.get_active_panel().marked.len();
    app.set_status(status::Status::CommandOutput(format!("{marked} selected")));
}

fn open_dialog(app: &mut App, command: crate::action::DialogCommand) {
    use crate::action::DialogCommand;
    match command {
        DialogCommand::ConfirmQuit => {
            app.set_overlay(Overlay::QuitConfirm(models::Confirm::default()));
        }
        DialogCommand::NewFile => app.set_overlay(Overlay::NewFileInput(String::new().into())),
        DialogCommand::ToggleHelp => toggle_help(app),
        DialogCommand::UserMenu => report(app, ops::menu::user::open),
        DialogCommand::WslManagement => ops::wsl::open_manager(app),
        DialogCommand::Mkdir => app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder())),
        DialogCommand::Menu => open_menu(app),
        DialogCommand::Information => {
            let directory = app.get_active_panel().path.clone();
            let stats = crate::stats::analyse(&directory);
            app.set_overlay(Overlay::Information(Box::new(stats)));
        }
    }
}

fn refresh(app: &mut App) {
    match crate::utils::refresh_panels(app) {
        Ok(()) => app.set_status(status::Status::PanelsRefreshed),
        Err(error) => app.set_status(status::Status::RefreshFailed(error.to_string())),
    }
}
