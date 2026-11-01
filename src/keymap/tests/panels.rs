//! Purpose: the `panels` table against the F9 menu — every row of the menu
//! is an action with a name, and a row's printed key is its first default.

use super::*;
use crate::menu::{Entry, Heading, Menu};
use crate::test_support::chord;

fn menu_rows() -> Vec<(Heading, &'static Entry)> {
    Heading::ALL
        .into_iter()
        .flat_map(|heading| {
            let rows = Menu::open(heading).entries().iter();
            rows.map(move |entry| (heading, entry))
        })
        .collect()
}

fn named(action: Action) -> Option<&'static Row<Action>> {
    tables::PANELS.rows().find(|row| row.command == action)
}

#[test]
fn every_menu_row_is_an_action_with_a_name() {
    for (heading, entry) in menu_rows() {
        let Some(action) = entry.action() else {
            continue;
        };
        assert!(
            named(action).is_some(),
            "{heading:?}: {:?} runs {action:?}, which has no name",
            crate::menu::label_of(entry)
        );
    }
}

/// Far's "Gray +" is the plain `+`: terminals cannot tell the pad's keys
/// from the row's.
#[test]
fn the_key_a_menu_row_prints_is_the_first_default_of_its_action() {
    let keymap = Keymap::default();
    for (heading, entry) in menu_rows() {
        let Entry::Item {
            shortcut, action, ..
        } = entry
        else {
            continue;
        };
        let printed = shortcut.strip_prefix("Gray ").unwrap_or(shortcut);
        let first = keymap.panels.keys(*action).next();
        let expected = (!printed.is_empty()).then(|| chord(printed));
        assert_eq!(first, expected, "{heading:?}: {shortcut:?}");
    }
}

#[test]
fn the_function_keys_are_bound_with_each_modifier_as_far_binds_them() {
    let panels = Keymap::default().panels;
    assert_eq!(panels.command(chord("F5")), Some(Action::Copy));
    assert_eq!(panels.command(chord("Shift+F5")), None, "exact matching");
    assert_eq!(panels.command(chord("F6")), Some(Action::Move));
    assert_eq!(panels.command(chord("Shift+F6")), Some(Action::Rename));
    assert_eq!(panels.command(chord("Ctrl+Shift+F9")), None);
    assert_eq!(panels.command(chord("F10")), Some(Action::ConfirmQuit));
}
