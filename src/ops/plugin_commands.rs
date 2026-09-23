//! Purpose: Far's `F11` Plugin commands list — opening it, and running the
//! row chosen.

use std::io;

use crate::models::{App, Overlay, PluginCommand, PluginCommands};

/// F11: open the list.
pub fn open(app: &mut App) {
    app.set_overlay(Overlay::PluginCommands(PluginCommands::default()));
}

/// Enter on a row. The list closes first, so a row that opens a window of
/// its own is not closed again behind it.
pub fn run(app: &mut App, command: PluginCommand) -> io::Result<()> {
    app.close_overlay();
    match command {
        PluginCommand::McpStatus => super::viewer::open_mcp_status(app)?,
        PluginCommand::Sessions => super::sessions::open(app),
        PluginCommand::ImportNetBox => super::sessions::import::prompt(app),
    }
    Ok(())
}

#[cfg(test)]
mod tests;
