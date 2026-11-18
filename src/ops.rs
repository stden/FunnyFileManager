//! Canonical operation ownership for file, command, shell, and viewer actions.

pub mod apply;
pub mod archive;
pub mod associations;
pub mod attributes;
pub mod compare;
pub mod database;
pub mod describe;
pub mod disk_cleanup;
mod dispatch;
pub(crate) mod duplicates;
mod editor;
pub(crate) mod editor_lsp;
mod file;
pub mod filter;
pub mod findfile;
pub mod findfolder;
pub mod folder_history;
pub(crate) mod git_pull;
pub(crate) mod git_sync;
pub mod history_list;
pub mod hotplug;
pub mod link;
pub mod longnames;
pub(crate) mod lsp_editor;
pub(crate) mod mcp_install;
pub(crate) mod menu;
pub(crate) mod name_prompt;
pub mod packages;
mod panel;
pub(crate) mod panel_details;
pub mod panel_session;
pub mod panels;
pub mod path_env;
pub mod performance;
pub mod plugin_commands;
pub(crate) mod program;
pub mod prompts;
pub mod quickview;
pub mod remote;
pub mod screens;
pub mod sessions;
pub mod settings;
mod shell;
pub mod shortcuts;
pub mod sortmenu;
pub(crate) mod space_analysis;
mod stub;
pub(crate) mod system_open;
pub(crate) mod system_temp_cleanup;
pub mod tasks;
pub mod tree;
pub mod usermenu;
#[cfg(feature = "keepass")]
pub(crate) mod vault;
pub mod videomode;
pub mod view_history;
mod viewer;
pub(crate) mod volume_maintenance;
pub mod wipe;
pub mod wsl;

pub use dispatch::execute_command;
pub(crate) use editor::{
    change_editor_encoding, copy_editor_selection, cut_editor_selection, insert_editor_file_name,
    paste_into_editor, reload_editor_encoding, toggle_editor_code_page,
};
pub use file::{
    answer_existing, cancel_existing, copy_with, get_source_paths, handle_copy, handle_delete,
    handle_delete_confirm, handle_delete_with_cancel, handle_mkdir, handle_move, helper_main,
    retry_as_administrator, HELPER_FLAG,
};
pub(crate) use viewer::view_path;
pub use viewer::{handle_view, open_mcp_status};

pub(crate) use file::handle_create_file;

const CANCEL_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(25);

pub(super) fn should_poll_cancel(
    shown: bool,
    last_poll: &mut Option<std::time::Instant>,
    now: std::time::Instant,
) -> bool {
    if !shown {
        return false;
    }
    if last_poll.is_some_and(|last| now.saturating_duration_since(last) < CANCEL_POLL_INTERVAL) {
        return false;
    }
    *last_poll = Some(now);
    true
}

pub(super) fn live_cancel() -> impl FnMut(bool) -> bool {
    let mut last_poll: Option<std::time::Instant> = None;
    move |shown| {
        should_poll_cancel(shown, &mut last_poll, std::time::Instant::now())
            && crate::terminal::cancel_requested()
    }
}

pub(crate) mod cloudflare_dns;
pub(crate) mod network_drives;
#[cfg(test)]
mod tests;
pub mod things;
mod timestamps;
pub(crate) mod token_usage;
