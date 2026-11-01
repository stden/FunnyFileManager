//! Purpose: the keymap — the key bindings of every screen as one value, and
//! the tables of default keys it is built from.
//!
//! [`Keymap`] holds one [`Bindings`] per context of `ffm.yml`; a resolver is
//! handed its context's and looks a chord up there. [`tables`] names every
//! action of every context with its command, its default keys, where the
//! key comes from and what it does; [`Table::layered`] builds a context's
//! bindings from its table and the keys a file gives. Everything here is
//! pure: no I/O, no environment, no state beyond the values.

use crate::action::command_line::completion::CompletionCommand;
use crate::action::command_line::search::LineSearchCommand;
use crate::action::command_line::LineCommand;
use crate::action::editor::EditorCommand;
use crate::action::lsp_editor::LspListCommand;
use crate::action::netbox::NetboxCommand;
use crate::action::path_env::PathCommand;
use crate::action::tasks::search::TaskSearchCommand;
use crate::action::tasks::TaskCommand;
use crate::action::tree::TreeCommand;
use crate::action::vault::VaultCommand;
use crate::action::viewer::ViewerCommand;
use crate::action::wsl::WslCommand;
use crate::action::{Action, Bindings};

mod chords;
mod context;
mod layering;
mod row;
pub mod tables;

pub use context::Context;
pub use layering::{Conflict, Explicit, Layered};
pub use row::{row, ActionName, Group, Origin, Role, Row, Table};

/// The keys in force on every screen, one field per context of `ffm.yml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keymap {
    /// The file panels and the Ctrl+O console.
    pub panels: Bindings<Action>,
    /// The Ctrl+T tree, read before `panels`.
    pub tree_panel: Bindings<TreeCommand>,
    /// The `NetBox` sessions list and a remote panel, read before `panels`.
    pub netbox_panel: Bindings<NetboxCommand>,
    /// The `:` command line.
    pub command_line: Bindings<LineCommand>,
    /// Its Ctrl+R history search, read instead of `command_line`.
    pub command_line_search: Bindings<LineSearchCommand>,
    /// Its open completion list, read before `command_line`.
    pub command_line_completion: Bindings<CompletionCommand>,
    /// The F3 viewer; its scroll commands also drive the quick-view panel.
    pub viewer: Bindings<ViewerCommand>,
    /// The F4 editor.
    pub editor: Bindings<EditorCommand>,
    /// The Ctrl+W task list.
    pub task_list: Bindings<TaskCommand>,
    /// The task list's `/` bar.
    pub task_list_search: Bindings<TaskSearchCommand>,
    /// The Ctrl+Alt+W WSL manager.
    pub wsl_manager: Bindings<WslCommand>,
    /// Options → Language servers: the list and the view.
    pub language_servers: Bindings<LspListCommand>,
    /// Commands → Edit PATH.
    pub path_editor: Bindings<PathCommand>,
    /// The `KeePass` vault.
    pub vault: Bindings<VaultCommand>,
}

impl Default for Keymap {
    /// Every context with the default keys of its table: what runs when
    /// there is no `ffm.yml`, and what every test starts from.
    fn default() -> Self {
        Self {
            panels: tables::PANELS.defaults(),
            tree_panel: tables::TREE_PANEL.defaults(),
            netbox_panel: tables::NETBOX_PANEL.defaults(),
            command_line: tables::COMMAND_LINE.defaults(),
            command_line_search: tables::COMMAND_LINE_SEARCH.defaults(),
            command_line_completion: tables::COMMAND_LINE_COMPLETION.defaults(),
            viewer: tables::VIEWER.defaults(),
            editor: tables::EDITOR.defaults(),
            task_list: tables::TASK_LIST.defaults(),
            task_list_search: tables::TASK_LIST_SEARCH.defaults(),
            wsl_manager: tables::WSL_MANAGER.defaults(),
            language_servers: tables::LANGUAGE_SERVERS.defaults(),
            path_editor: tables::PATH_EDITOR.defaults(),
            vault: tables::VAULT.defaults(),
        }
    }
}

#[cfg(test)]
mod tests;
