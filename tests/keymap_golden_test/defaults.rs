//! Purpose: the keymap's defaults against the golden tables, before any
//! context reads its keys from the keymap — every key an old resolver
//! binds is a default with the same command, and every default is a key
//! the old resolver answers with that command.
//!
//! Step 5 of the design's migration plan switches the contexts one by one
//! and points the replay at the keymap; these two tests are what lets each
//! switch trust the tables it did not write.

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::fs;
use std::io;

use funny_file_manager::action::Bindings;
use funny_file_manager::keymap::Keymap;

use super::table;

/// A chord's canonical text and its command's debug form, as a golden
/// table writes both.
type Keys = BTreeMap<String, String>;

/// The one difference the design names (§6.3): the tree's old resolver
/// answers F5–F8 by key code with `NeedsFilePanel`; the tree's table has no
/// such action, because the refusal follows the panels action's scope.
const TREE_REFUSAL: &str = "NeedsFilePanel";

fn defaults<C: Copy + PartialEq + Debug>(bindings: &Bindings<C>) -> Keys {
    bindings
        .chords()
        .map(|(chord, command)| (chord.to_string(), format!("{command:?}")))
        .collect()
}

/// Every golden table beside the defaults it is held to; the quick view
/// reads the viewer's.
fn contexts() -> Vec<(&'static str, Keys)> {
    let keymap = Keymap::default();
    vec![
        ("panels", defaults(&keymap.panels)),
        ("tree_panel", defaults(&keymap.tree_panel)),
        ("netbox_panel", defaults(&keymap.netbox_panel)),
        ("quick_view", defaults(&keymap.viewer)),
        ("viewer", defaults(&keymap.viewer)),
        ("editor", defaults(&keymap.editor)),
        ("command_line", defaults(&keymap.command_line)),
        ("command_line_search", defaults(&keymap.command_line_search)),
        (
            "command_line_completion",
            defaults(&keymap.command_line_completion),
        ),
        ("task_list", defaults(&keymap.task_list)),
        ("task_list_search", defaults(&keymap.task_list_search)),
        ("wsl_manager", defaults(&keymap.wsl_manager)),
        ("language_servers", defaults(&keymap.language_servers)),
        ("path_editor", defaults(&keymap.path_editor)),
        ("vault", defaults(&keymap.vault)),
    ]
}

/// The rows of a golden table of the given classes, chord to result.
fn recorded(context: &str, classes: &[&str]) -> io::Result<Keys> {
    let text = fs::read_to_string(table::directory().join(format!("{context}.tsv")))?;
    let row = |line: &str| {
        let mut fields = line.split('\t');
        let (chord, class, result) = (fields.next()?, fields.next()?, fields.next()?);
        classes
            .contains(&class)
            .then(|| (chord.to_owned(), result.to_owned()))
    };
    Ok(text.lines().filter_map(row).collect())
}

#[test]
fn every_context_of_the_tables_is_held_to_its_defaults() {
    let held: Vec<&str> = contexts().iter().map(|(name, _)| *name).collect();
    let tables: Vec<&str> = super::CONTEXTS.iter().map(|context| context.name).collect();
    assert_eq!(held, tables);
}

#[test]
fn every_key_an_old_resolver_binds_is_a_default_with_the_same_command() {
    for (context, defaults) in contexts() {
        let bound = recorded(context, &["bound"]).expect("the golden table");
        assert!(!bound.is_empty(), "{context} binds nothing");
        for (chord, result) in bound {
            if context == "tree_panel" && result == TREE_REFUSAL {
                assert_eq!(defaults.get(&chord), None, "tree_panel: {chord}");
                continue;
            }
            assert_eq!(defaults.get(&chord), Some(&result), "{context}: {chord}");
        }
    }
}

/// A default may stand in a `lenient` row: the task list's Ctrl+T is its
/// own default and also what `t` with any modifier used to give.
#[test]
fn every_default_is_a_key_the_old_resolver_answers_with_its_command() {
    for (context, defaults) in contexts() {
        let answered = recorded(context, &["bound", "lenient"]).expect("the golden table");
        for (chord, command) in defaults {
            if context == "quick_view" && command == "Close" {
                continue;
            }
            assert_eq!(answered.get(&chord), Some(&command), "{context}: {chord}");
        }
    }
}

#[test]
fn the_trees_refusal_is_the_four_keys_of_the_file_operations() {
    let refused: Vec<String> = recorded("tree_panel", &["bound"])
        .expect("the golden table")
        .into_iter()
        .filter(|(_, result)| result == TREE_REFUSAL)
        .map(|(chord, _)| chord)
        .collect();
    assert_eq!(refused, ["F5", "F6", "F7", "F8"]);
}
