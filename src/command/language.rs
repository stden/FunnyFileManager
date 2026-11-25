//! Purpose: The typed languages keys, menus and the command line translate into.
//!
//! Data only, in the state layer: translation tables live in [`crate::action`]
//! and [`crate::menu`], interpreters in `ops` and `popup`. Both sides depend on
//! these types without going up a layer.

mod action;
mod commands;
mod menu;
mod wsl;

pub use action::Action;
pub use commands::{
    DialogCommand, FileCommand, Navigation, PanelCommand, SelectionCommand, ToolCommand,
};
pub use menu::{MenuCommand, SettingsPage};
pub use wsl::WslCommand;
