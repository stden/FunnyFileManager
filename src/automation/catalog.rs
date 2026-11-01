//! Purpose: Export every named keymap action as a typed automation invocation.

use std::fmt;

use serde::Serialize;
use serde_json::Value;

use crate::action::{
    command_line::{completion::CompletionCommand, search::LineSearchCommand, LineCommand},
    editor::EditorCommand,
    lsp_editor::LspListCommand,
    netbox::NetboxCommand,
    path_env::PathCommand,
    tasks::{search::TaskSearchCommand, TaskCommand},
    tree::TreeCommand,
    vault::VaultCommand,
    viewer::ViewerCommand,
    wsl::WslCommand,
    Action,
};
use crate::keymap::{tables, Context, Table};

/// Stable action name within its context, sourced from the canonical table.
#[derive(Clone, Copy, Serialize)]
#[serde(transparent)]
struct ActionId(&'static str);

#[derive(Serialize)]
struct Descriptor {
    context: &'static str,
    action: ActionId,
    summary: &'static str,
    default_keys: Vec<String>,
}

/// A failed catalogue lookup never carries arbitrary caller-supplied text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookupError {
    UnknownContext,
    UnknownAction,
}

impl fmt::Display for LookupError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(match self {
            Self::UnknownContext => "unknown automation context",
            Self::UnknownAction => "unknown action for this context",
        })
    }
}

impl std::error::Error for LookupError {}

macro_rules! catalogue {
    ($($variant:ident($command:ty), $field:ident, $table:ident;)+) => {
        /// A resolved command: no string dispatch or debug serialization at runtime.
        #[derive(Clone, Copy)]
        pub enum Invocation {
            $($variant($command),)+
        }

        impl Invocation {
            pub const fn context(self) -> Context {
                match self {
                    $(Self::$variant(_) => Context::$variant,)+
                }
            }
        }

        pub fn entries() -> Value {
            let mut rows = Vec::new();
            $(append(&mut rows, &tables::$table);)+
            serde_json::json!(rows)
        }

        pub fn resolve(context: &str, action: &str) -> Result<Invocation, LookupError> {
            match context {
                $(stringify!($field) => lookup(&tables::$table, action).map(Invocation::$variant),)+
                _ => Err(LookupError::UnknownContext),
            }
        }
    };
}

catalogue! {
    Panels(Action), panels, PANELS;
    TreePanel(TreeCommand), tree_panel, TREE_PANEL;
    NetboxPanel(NetboxCommand), netbox_panel, NETBOX_PANEL;
    CommandLine(LineCommand), command_line, COMMAND_LINE;
    CommandLineSearch(LineSearchCommand), command_line_search, COMMAND_LINE_SEARCH;
    CommandLineCompletion(CompletionCommand), command_line_completion, COMMAND_LINE_COMPLETION;
    Viewer(ViewerCommand), viewer, VIEWER;
    Editor(EditorCommand), editor, EDITOR;
    TaskList(TaskCommand), task_list, TASK_LIST;
    TaskListSearch(TaskSearchCommand), task_list_search, TASK_LIST_SEARCH;
    WslManager(WslCommand), wsl_manager, WSL_MANAGER;
    LanguageServers(LspListCommand), language_servers, LANGUAGE_SERVERS;
    PathEditor(PathCommand), path_editor, PATH_EDITOR;
    Vault(VaultCommand), vault, VAULT;
}

fn append<C>(descriptors: &mut Vec<Descriptor>, table: &Table<C>) {
    descriptors.extend(table.rows().map(|row| Descriptor {
        context: table.context.name(),
        action: ActionId(row.name.as_str()),
        summary: row.summary,
        default_keys: row.defaults.iter().map(ToString::to_string).collect(),
    }));
}

fn lookup<C: Copy>(table: &Table<C>, name: &str) -> Result<C, LookupError> {
    table
        .rows()
        .find(|row| row.name.as_str() == name)
        .map(|row| row.command)
        .ok_or(LookupError::UnknownAction)
}

#[cfg(test)]
mod tests;
