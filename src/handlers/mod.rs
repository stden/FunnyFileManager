//! Backward-compatible facade for operation and popup APIs.

pub use crate::ops::{
    execute_command, handle_copy, handle_delete, handle_delete_confirm, handle_mkdir, handle_move,
    handle_view, open_mcp_status,
};
pub use crate::popup::process_popup_input;

#[doc(hidden)]
pub mod dispatch {
    pub use crate::ops::{execute_command, handle_delete};
}

#[doc(hidden)]
pub mod file_ops {
    pub use crate::ops::{
        handle_copy, handle_delete, handle_delete_confirm, handle_mkdir, handle_move, handle_view,
    };

    #[cfg(test)]
    pub(crate) use crate::ops::{
        format_operation_status, get_source_paths, rename_or_fail, FORCE_RENAME_FAILURE,
        RENAME_TEST_MUTEX,
    };
}

#[doc(hidden)]
pub mod mcp {
    pub use crate::ops::open_mcp_status;
}

#[doc(hidden)]
pub mod shell {
    #[cfg(test)]
    pub(crate) use crate::ops::{run_shell_command, windows_shell_args};
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_copy_move;
#[cfg(test)]
mod tests_copy_move_edge_cases;
#[cfg(test)]
mod tests_delete_and_edit;
#[cfg(test)]
mod tests_delete_confirm_shell;
#[cfg(test)]
mod tests_drive_selection;
#[cfg(test)]
mod tests_marked_ops_and_errors;
#[cfg(test)]
mod tests_marks_and_bookmarks;
#[cfg(test)]
mod tests_misc_coverage;
#[cfg(test)]
mod tests_new_file_and_drives;
#[cfg(test)]
mod tests_popup_command_line;
#[cfg(test)]
mod tests_popup_input_keys;
#[cfg(test)]
mod tests_search_and_commands;
#[cfg(test)]
mod tests_shell_commands;
#[cfg(test)]
mod tests_source_paths_fallbacks;
#[cfg(test)]
mod tests_transfer_guards;
