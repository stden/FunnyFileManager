//! Purpose: the key tables read as facts — what each row says apart from
//! the type of its command — and the fourteen contexts they cover.

use std::fmt::Debug;

use super::*;
use crate::action::KeyChord;

mod app;
mod defaults;
mod exits;
mod lists;
mod names;
mod panels;

/// A row without the type of its command, and the keys the default keymap
/// gives it.
pub(super) struct Facts {
    pub group: &'static str,
    pub name: ActionName,
    /// The command in its debug form.
    pub command: String,
    pub defaults: &'static [KeyChord],
    pub origin: Origin,
    pub summary: &'static str,
    pub role: Role,
    pub in_force: Vec<KeyChord>,
}

pub(super) struct Screen {
    pub context: Context,
    pub rows: Vec<Facts>,
    /// The chords in force by default.
    pub bound: usize,
}

fn screen<C: Copy + PartialEq + Debug>(table: &Table<C>, bindings: &Bindings<C>) -> Screen {
    let group = |group: &'static Group<C>| {
        group.rows.iter().map(|row| Facts {
            group: group.title,
            name: row.name,
            command: format!("{:?}", row.command),
            defaults: row.defaults,
            origin: row.origin,
            summary: row.summary,
            role: row.role,
            in_force: bindings.keys(row.command).collect(),
        })
    };
    Screen {
        context: table.context,
        rows: table.groups.iter().flat_map(group).collect(),
        bound: bindings.chords().count(),
    }
}

/// Every table beside the default keymap's field for it.
pub(super) fn screens() -> Vec<Screen> {
    let keymap = Keymap::default();
    vec![
        screen(&tables::PANELS, &keymap.panels),
        screen(&tables::TREE_PANEL, &keymap.tree_panel),
        screen(&tables::NETBOX_PANEL, &keymap.netbox_panel),
        screen(&tables::COMMAND_LINE, &keymap.command_line),
        screen(&tables::COMMAND_LINE_SEARCH, &keymap.command_line_search),
        screen(
            &tables::COMMAND_LINE_COMPLETION,
            &keymap.command_line_completion,
        ),
        screen(&tables::VIEWER, &keymap.viewer),
        screen(&tables::EDITOR, &keymap.editor),
        screen(&tables::TASK_LIST, &keymap.task_list),
        screen(&tables::TASK_LIST_SEARCH, &keymap.task_list_search),
        screen(&tables::WSL_MANAGER, &keymap.wsl_manager),
        screen(&tables::LANGUAGE_SERVERS, &keymap.language_servers),
        screen(&tables::PATH_EDITOR, &keymap.path_editor),
        screen(&tables::VAULT, &keymap.vault),
    ]
}

pub(super) fn screen_of(context: Context) -> Screen {
    screens()
        .into_iter()
        .find(|screen| screen.context == context)
        .unwrap_or_else(|| panic!("{context} has no table"))
}

impl Screen {
    pub fn row(&self, name: &str) -> &Facts {
        self.rows
            .iter()
            .find(|row| row.name.as_str() == name)
            .unwrap_or_else(|| panic!("{} has no action {name}", self.context))
    }
}

pub(super) fn texts(chords: &[KeyChord]) -> Vec<String> {
    chords.iter().map(ToString::to_string).collect()
}

/// The items that stand in the list more than once, each named once.
pub(super) fn repeated<T: Ord>(mut items: Vec<T>) -> Vec<T> {
    items.sort_unstable();
    let mut twice = Vec::new();
    while let Some(item) = items.pop() {
        if items.last() == Some(&item) && twice.last() != Some(&item) {
            twice.push(item);
        }
    }
    twice.reverse();
    twice
}

#[test]
fn repeated_items_are_named_once_each() {
    assert_eq!(repeated(vec![3, 1, 2]), Vec::<i32>::new());
    assert_eq!(repeated(vec![3, 1, 3, 2, 1, 3]), [1, 3]);
    assert_eq!(repeated(Vec::<i32>::new()), Vec::<i32>::new());
}

#[test]
fn every_context_has_its_table_in_the_files_order() {
    let contexts: Vec<Context> = screens().iter().map(|screen| screen.context).collect();
    assert_eq!(contexts, Context::ALL);
}

#[test]
fn the_contexts_are_named_as_the_file_names_its_sections() {
    let names: Vec<String> = Context::ALL.iter().map(ToString::to_string).collect();
    assert_eq!(
        names,
        [
            "panels",
            "tree_panel",
            "netbox_panel",
            "command_line",
            "command_line_search",
            "command_line_completion",
            "viewer",
            "editor",
            "task_list",
            "task_list_search",
            "wsl_manager",
            "language_servers",
            "path_editor",
            "vault",
        ]
    );
    assert_eq!(Context::Vault.name(), "vault");
}

/// A row lost or added is a change to the file every user has.
#[test]
fn the_number_of_actions_of_each_context_is_pinned() {
    let counts: Vec<(&str, usize)> = screens()
        .iter()
        .map(|screen| (screen.context.name(), screen.rows.len()))
        .collect();
    assert_eq!(
        counts,
        [
            ("panels", 116),
            ("tree_panel", 15),
            ("netbox_panel", 12),
            ("command_line", 29),
            ("command_line_search", 4),
            ("command_line_completion", 4),
            ("viewer", 11),
            ("editor", 34),
            ("task_list", 42),
            ("task_list_search", 1),
            ("wsl_manager", 9),
            ("language_servers", 14),
            ("path_editor", 12),
            ("vault", 18),
        ]
    );
}
