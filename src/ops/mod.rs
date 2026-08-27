//! Canonical operation ownership for file, command, shell, and viewer actions.

mod dispatch;
mod file;
#[allow(clippy::redundant_pub_crate)]
mod shell;
mod viewer;

pub use dispatch::execute_command;
pub use file::{handle_copy, handle_delete, handle_delete_confirm, handle_mkdir, handle_move};
pub use viewer::{handle_view, open_mcp_status};

pub(crate) use file::handle_create_file;

#[cfg(test)]
pub(crate) use file::{
    format_operation_status, get_source_paths, rename_or_fail, FORCE_RENAME_FAILURE,
    RENAME_TEST_MUTEX,
};
#[cfg(test)]
pub(crate) use shell::{run_shell_command, windows_shell_args};
