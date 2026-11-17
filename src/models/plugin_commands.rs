//! Purpose: Far's `F11` "Plugin commands" list — the rows this program's
//! plugins offer, and the cursor.
//!
//! State only: what each row does is `ops::plugin_commands::run`'s, and the
//! keys are `popup::plugin_commands`'. The rows are a fixed table rather
//! than data, because they are this program's own commands.

use super::{RowCursor, RowMotion};

/// One row of the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginCommand {
    /// What `F11` used to open directly.
    McpStatus,
    /// The stored-sessions panel.
    Sessions,
    /// Import sessions from a `.netbox` file, asking for the file first.
    ImportNetBox,
}

impl PluginCommand {
    /// In the order the list shows them.
    pub const ALL: [Self; 3] = [Self::McpStatus, Self::Sessions, Self::ImportNetBox];

    /// The row's text. The import row ends in `…`, Far's mark for a command
    /// that asks a question before it acts.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::McpStatus => "MCP status",
            Self::Sessions => "NetBox: stored sessions",
            Self::ImportNetBox => "NetBox: import from .netbox file…",
        }
    }
}

/// The list on screen and where the cursor is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PluginCommands {
    cursor: RowCursor,
}

impl PluginCommands {
    #[must_use]
    pub const fn cursor(&self) -> usize {
        self.cursor.index()
    }

    /// The row under the cursor. The cursor never leaves the table, and a
    /// table that ever came up empty would fall back to its first row.
    #[must_use]
    pub fn selected(&self) -> PluginCommand {
        PluginCommand::ALL
            .get(self.cursor.index())
            .copied()
            .unwrap_or(PluginCommand::McpStatus)
    }

    pub fn move_cursor(&mut self, motion: RowMotion) {
        self.cursor = self.cursor.moved(motion, PluginCommand::ALL.len());
    }
}
