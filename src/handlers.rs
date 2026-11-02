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
}

#[doc(hidden)]
pub mod mcp {
    pub use crate::ops::open_mcp_status;
}

#[doc(hidden)]
pub mod shell {}
