//! Purpose: the keys a row carries: shortcuts resolve to the action they name, and the constructors.
use super::*;

// ─── the shortcuts a menu prints must be real ───────────────────────────────

/// Every key a menu row names must actually be that key.
///
/// The bug this exists for: `Alt+F8` was offered for "History", but the key
/// table matches `F(8)` without checking modifiers, so it resolved to the
/// delete confirmation. A row that reads as harmless opened a destructive
/// dialog — the worst shape a wrong shortcut can take.
///
/// The invariant is general rather than a table of expected actions, which
/// would just be the menu written twice: if a row's key carries a modifier,
/// resolving it must differ from resolving the same key *without* the
/// modifier. When they agree, the modifier is being dropped.
#[test]
fn a_modified_shortcut_must_not_resolve_like_an_unmodified_one() {
    use crossterm::event::{KeyEvent, KeyModifiers};

    use crate::action::resolve;

    for heading in Heading::ALL {
        for entry in Menu::open(heading).entries() {
            let Some(shortcut) = entry.key() else {
                continue;
            };
            if shortcut.modifiers == KeyModifiers::NONE {
                continue;
            }
            let with = resolve(KeyEvent::new(shortcut.code, shortcut.modifiers));
            let without = resolve(KeyEvent::new(shortcut.code, KeyModifiers::NONE));
            assert_ne!(
                with,
                without,
                "{heading:?}: {:?} offers {:?}+{:?}, but the modifier is ignored — \
                 it does whatever the bare key does ({without:?})",
                label_of(entry),
                shortcut.modifiers,
                shortcut.code,
            );
        }
    }
}

/// …and no row may reach a destructive action unless it says so.
///
/// A second, blunter guard on the same failure: whatever else goes wrong, the
/// only menu row allowed to open the delete confirmation is the one labelled
/// Delete.
#[test]
fn only_the_delete_row_can_reach_the_delete_dialog() {
    use crossterm::event::KeyEvent;

    use crate::action::{resolve, Action};

    for heading in Heading::ALL {
        for entry in Menu::open(heading).entries() {
            let Some(shortcut) = entry.key() else {
                continue;
            };
            let label = label_of(entry);
            let action = resolve(KeyEvent::new(shortcut.code, shortcut.modifiers));
            if matches!(action, Action::DeleteConfirm) {
                assert!(
                    label.contains("Delete"),
                    "{heading:?}: {label:?} reaches the delete dialog"
                );
            }
        }
    }
}

/// Every row that claims a key must reach something that does work — not the
/// quick-search reset every unrecognised `Ctrl+letter` falls into.
#[test]
fn no_row_falls_through_to_the_quick_search_reset() {
    use crossterm::event::KeyEvent;

    use crate::action::{resolve, Action};

    for heading in Heading::ALL {
        for entry in Menu::open(heading).entries() {
            let Some(shortcut) = entry.key() else {
                continue;
            };
            let action = resolve(KeyEvent::new(shortcut.code, shortcut.modifiers));
            assert!(
                !matches!(
                    action,
                    Action::QuickSearchClear | Action::CommandLineChar(_)
                ),
                "{heading:?}: {:?} claims a key that is not recognised — it falls \
                 through to {action:?}",
                label_of(entry),
            );
        }
    }
}

// ─── the two constructors, and what makes a row grey ────────────────────────

/// The whole "greyed" mechanism is the difference between these two: a row
/// built by `item` carries a key and answers Enter, one built by `planned`
/// carries none and is skipped by the cursor. Called at runtime here because
/// the tables call them at compile time, and a constructor that swapped the
/// two would produce a menu that looks right and does nothing.
#[test]
fn shortcut_and_command_items_are_both_executable() {
    use crossterm::event::KeyCode;

    use super::{Entry, Shortcut};

    let working = Entry::item("&Delete", "F8", Shortcut::plain(KeyCode::F(8)));
    assert!(working.is_selectable(), "a row with a key can be chosen");
    assert_eq!(working.key().map(|key| key.code), Some(KeyCode::F(8)));
    assert_eq!(label_of(&working), "&Delete");

    let command = Entry::command("Add &to archive", "Shift+F1", MenuCommand::Archive);
    assert!(command.is_selectable());
    assert_eq!(command.menu_command(), Some(MenuCommand::Archive));
    assert_eq!(label_of(&command), "Add &to archive");
}

/// Each shortcut constructor must set its own modifier and no other. A
/// copy-paste slip here is how `Alt+F8` came to mean `F8` — the bug that
/// pointed a menu row at the delete confirmation.
#[test]
fn each_shortcut_constructor_sets_exactly_its_own_modifier() {
    use crossterm::event::{KeyCode, KeyModifiers};

    use super::Shortcut;

    for (built, expected) in [
        (Shortcut::plain(KeyCode::F(3)), KeyModifiers::NONE),
        (Shortcut::alt(KeyCode::F(3)), KeyModifiers::ALT),
        (Shortcut::shift(KeyCode::F(3)), KeyModifiers::SHIFT),
    ] {
        assert_eq!(built.code, KeyCode::F(3), "the key itself is unchanged");
        assert_eq!(built.modifiers, expected);
    }

    let control = Shortcut::ctrl('r');
    assert_eq!(control.code, KeyCode::Char('r'));
    assert_eq!(control.modifiers, KeyModifiers::CONTROL);
}
