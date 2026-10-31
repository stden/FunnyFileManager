//! Purpose: body-row click handlers for the remaining modal overlays.

use crossterm::event::{KeyCode, KeyEvent};

use crate::models::{
    ArchiveCommand, ArchiveField, ArchivePrompt, AttributesDialog, AttributesField, CleanupField,
    DiskCleanupDialog, ExtractField, FindFile, FindFileField, Flag, McpInstallDialog,
    McpInstallField, McpInstallProgress, Overlay, SessionEdit, SessionField,
};
use crate::search::{SearchField, SearchPrompt};

pub(super) fn click_more_row(overlay: &mut Overlay, index: usize) -> Option<KeyEvent> {
    match overlay {
        Overlay::Editor(editor)
            if matches!(editor.modal(), Some(crate::models::EditorModal::Search(_))) =>
        {
            if let Some(prompt) = &mut editor.search.prompt {
                click_editor_search_row(prompt, index);
            }
        }
        Overlay::Remote(dialog) => super::remote_row(&mut dialog.0, index),
        Overlay::ArchivePrompt(prompt) => click_archive_row(prompt, index),
        Overlay::ExtractPrompt(prompt) if index == 1 => prompt.field = ExtractField::Destination,
        Overlay::ArchiveCommands(menu) if index < ArchiveCommand::ALL.len() => {
            menu.cursor = index;
            return Some(KeyCode::Enter.into());
        }
        Overlay::Attributes(dialog) => click_attributes_row(dialog, index),
        Overlay::McpInstall(dialog) => click_mcp_install_row(dialog, index),
        Overlay::FindFile(dialog) => click_find_file_row(dialog, index),
        Overlay::SessionEdit(dialog) => click_session_edit_row(dialog, index),
        Overlay::DiskCleanup(dialog) => click_disk_cleanup_row(dialog, index),
        Overlay::MaskPrompt(prompt) if index == 1 => prompt.focus_cancel(false),
        Overlay::ApplyCommand(prompt) if index == 1 => prompt.focus_cancel(false),
        Overlay::Describe(prompt) if index == 1 => prompt.focus_cancel(false),
        Overlay::InputPrompt(prompt) if index == 1 && prompt.on_cancel() => {
            prompt.toggle_button();
        }
        _ => {}
    }
    None
}

fn click_editor_search_row(prompt: &mut SearchPrompt, index: usize) {
    match index {
        1 => prompt.field = SearchField::Query,
        3 => {
            prompt.field = SearchField::Case;
            prompt.toggle();
        }
        4 => {
            prompt.field = SearchField::Words;
            prompt.toggle();
        }
        _ => {}
    }
}

fn click_archive_row(prompt: &mut ArchivePrompt, index: usize) {
    match index {
        1 => prompt.field = ArchiveField::Name,
        3 => prompt.field = ArchiveField::Format,
        5 => {
            prompt.field = ArchiveField::DeleteAfter;
            prompt.toggle();
        }
        _ => {}
    }
}

fn click_attributes_row(dialog: &mut AttributesDialog, index: usize) {
    const ABOVE_GROUPS: usize = 3;
    let group_count = Flag::GROUPS.len();
    if let Some(group) = index.checked_sub(ABOVE_GROUPS).filter(|&g| g < group_count) {
        let flag_index: usize = Flag::GROUPS
            .get(..group)
            .unwrap_or_default()
            .iter()
            .map(|g| g.flags.len())
            .sum();
        if flag_index < dialog.flags.len() {
            dialog.field = AttributesField::Flag(flag_index);
            dialog.toggle();
        }
        return;
    }
    let after_groups = ABOVE_GROUPS + group_count;
    if index == after_groups + 1 {
        dialog.field = AttributesField::Modified;
    } else if index == after_groups + 2 {
        dialog.field = AttributesField::Accessed;
    } else if dialog.has_directories && index == after_groups + 4 {
        dialog.field = AttributesField::Subfolders;
        dialog.toggle();
    }
}

fn click_mcp_install_row(dialog: &mut McpInstallDialog, index: usize) {
    if dialog.progress != McpInstallProgress::Choosing {
        return;
    }
    let server_len = crate::mcp_clients::server_lines(&dialog.server).len();
    if index == server_len + 1 {
        dialog.field = McpInstallField::Scope;
        return;
    }
    let first_client = server_len + 4;
    if let Some(row) = index
        .checked_sub(first_client)
        .and_then(|i| dialog.rows.get(i))
    {
        if dialog.is_enabled(row) {
            dialog.field = McpInstallField::Client(row.status.client);
            dialog.toggle();
        }
    }
}

fn click_find_file_row(dialog: &mut FindFile, index: usize) {
    let target = match index {
        1 => FindFileField::Mask,
        3 => FindFileField::Containing,
        _ => return,
    };
    while dialog.field() != target {
        dialog.tab(true);
    }
}

fn click_session_edit_row(dialog: &mut SessionEdit, index: usize) {
    const BODY_FIELDS: [SessionField; 8] = [
        SessionField::Name,
        SessionField::Protocol,
        SessionField::Host,
        SessionField::Port,
        SessionField::User,
        SessionField::Auth,
        SessionField::KeyFile,
        SessionField::RemoteDir,
    ];
    let Some(&target) = BODY_FIELDS.get(index) else {
        return;
    };
    if !dialog.is_enabled(target) {
        return;
    }
    while dialog.field() != target {
        dialog.tab(true);
    }
}

fn click_disk_cleanup_row(dialog: &mut DiskCleanupDialog, index: usize) {
    let Some(candidate) = dialog.candidates.get_mut(index) else {
        return;
    };
    if !candidate.exists() {
        return;
    }
    dialog.field = CleanupField::Item(index);
    candidate.toggle_checked();
}
