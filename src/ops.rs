//! Canonical operation ownership for file, command, shell, and viewer actions.

pub mod apply;
pub mod archive;
pub mod associations;
pub mod attributes;
pub mod compare;
pub mod describe;
pub mod disk_cleanup;
mod dispatch;
mod editor;
pub(crate) mod editor_lsp;
mod file;
pub mod filter;
pub mod findfile;
pub mod findfolder;
pub mod folder_history;
pub(crate) mod git_pull;
pub mod history_list;
pub mod hotplug;
pub mod link;
pub mod longnames;
pub(crate) mod lsp_editor;
pub(crate) mod mcp_install;
pub(crate) mod menu;
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
mod stub;
pub(crate) mod system_open;
pub mod tasks;
pub mod tree;
pub mod usermenu;
// Unconditional: the file carries its own `#[cfg(feature = "keepass")]` pair,
// so the `Enter` and `F3` call sites stay feature-free.
pub(crate) mod vault;
pub mod videomode;
pub mod view_history;
mod viewer;
pub mod wipe;
pub mod wsl;

pub use dispatch::execute_command;
pub(crate) use editor::{
    change_editor_encoding, copy_editor_selection, insert_editor_file_name, paste_into_editor,
    reload_editor_encoding,
};
pub use file::{
    answer_existing, cancel_existing, copy_with, get_source_paths, handle_copy, handle_delete,
    handle_delete_confirm, handle_mkdir, handle_move, helper_main, retry_as_administrator,
    HELPER_FLAG,
};
pub(crate) use viewer::view_path;
pub use viewer::{handle_view, open_mcp_status};

pub(crate) use file::handle_create_file;

#[cfg(test)]
mod tests;
mod timestamps;
