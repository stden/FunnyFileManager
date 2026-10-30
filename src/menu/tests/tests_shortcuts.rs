//! Purpose: the shortcut a row prints is a key of the action it holds, and what a row is.
use super::*;

use crate::action::{Action, KeyChord};
use crate::test_support::panels_action as resolve;

/// The key text beside a row, as a chord: Far's "Gray +" is the plain `+`
/// here, since terminals cannot tell the pad's keys from the row's.
fn printed_chord(entry: &Entry) -> Option<KeyChord> {
    let Entry::Item { shortcut, .. } = entry else {
        return None;
    };
    if shortcut.is_empty() {
        return None;
    }
    let text = shortcut.strip_prefix("Gray ").unwrap_or(shortcut);
    Some(
        text.parse()
            .unwrap_or_else(|error| panic!("{shortcut:?}: {error}")),
    )
}

// ─── the shortcuts a menu prints must be real ───────────────────────────────

/// Every key a menu row prints must resolve to the action the row holds.
///
/// The bug this exists for: `Alt+F8` was offered for "History", but the key
/// table matched `F(8)` without checking modifiers, so it resolved to the
/// delete confirmation. A row that reads as harmless opened a destructive
/// dialog — the worst shape a wrong shortcut can take. The row now holds the
/// action and prints the key; this is what keeps the two from drifting.
#[test]
fn every_printed_shortcut_resolves_to_the_rows_action() {
    for heading in Heading::ALL {
        for entry in Menu::open(heading).entries() {
            let Some(chord) = printed_chord(entry) else {
                continue;
            };
            assert_eq!(
                resolve(chord.to_event()),
                entry.action().expect("a row with a shortcut is an item"),
                "{heading:?}: {:?} prints {chord}",
                label_of(entry),
            );
        }
    }
}

/// …and no row may hold a destructive action unless it says so.
///
/// A second, blunter guard on the same failure: whatever else goes wrong, the
/// only menu row allowed to open the delete confirmation is the one labelled
/// Delete.
#[test]
fn only_the_delete_row_can_reach_the_delete_dialog() {
    for heading in Heading::ALL {
        for entry in Menu::open(heading).entries() {
            if entry.action() == Some(Action::DeleteConfirm) {
                let label = label_of(entry);
                assert!(
                    label.contains("Delete"),
                    "{heading:?}: {label:?} reaches the delete dialog"
                );
            }
        }
    }
}

/// Every row must hold something that does work — not the quick-search reset
/// every unrecognised `Ctrl+letter` falls into, nor a typed character.
#[test]
fn no_row_holds_the_quick_search_reset() {
    for heading in Heading::ALL {
        for entry in Menu::open(heading).entries() {
            assert!(
                !matches!(
                    entry.action(),
                    Some(Action::QuickSearchClear | Action::CommandLineChar(_))
                ),
                "{heading:?}: {:?} holds {:?}",
                label_of(entry),
                entry.action(),
            );
        }
    }
}

// ─── what a row is ──────────────────────────────────────────────────────────

/// An item can be chosen and holds its action; a separator can be neither
/// chosen nor run. Called at runtime here because the tables call the
/// constructor at compile time, and a constructor that dropped the action
/// would produce a menu that looks right and does nothing.
#[test]
fn an_item_holds_its_action_and_a_separator_holds_none() {
    let working = Entry::item("&Delete", "F8", Action::DeleteConfirm);
    assert!(
        working.is_selectable(),
        "a row with an action can be chosen"
    );
    assert_eq!(working.action(), Some(Action::DeleteConfirm));
    assert_eq!(label_of(&working), "&Delete");

    assert!(!Entry::Separator.is_selectable());
    assert_eq!(Entry::Separator.action(), None);
}
