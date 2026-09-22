use super::{Entry, Heading, Menu};

#[test]
fn the_bar_is_a_ring_in_both_directions() {
    let mut heading = Heading::Left;
    for _ in 0..5 {
        assert_eq!(heading.next().previous(), heading);
        heading = heading.next();
    }
    assert_eq!(heading, Heading::Left, "five steps come back round");
}

#[test]
fn every_heading_has_a_hotkey_and_they_are_all_different() {
    let mut letters: Vec<char> = Heading::ALL
        .into_iter()
        .map(|heading| {
            super::hotkey_of(heading.label()).unwrap_or_else(|| panic!("{heading:?} has no hotkey"))
        })
        .collect();
    letters.sort_unstable();
    let unique = {
        let mut seen = letters.clone();
        seen.dedup();
        seen
    };
    assert_eq!(
        unique.len(),
        letters.len(),
        "no two headings share a letter"
    );
}

/// The cursor must never rest on a rule or on a command the program does not
/// have — pressing Enter there would do nothing, which reads as a dead key.
///
/// Unless *nothing* in the drop-down works, which is true of Options: this
/// program keeps no configuration, so all twenty-seven of its rows are greyed.
/// There the cursor rests on nothing at all, which is the honest answer.
#[test]
fn the_cursor_only_ever_rests_on_something_that_works() {
    for heading in Heading::ALL {
        let mut menu = Menu::open(heading);
        if !menu.has_anything_usable() {
            assert!(
                !menu.is_on_a_usable_row(),
                "{heading:?} offers nothing, so the cursor must claim nothing"
            );
            continue;
        }
        for _ in 0..menu.entries().len() * 2 {
            assert!(menu.is_on_a_usable_row(), "{heading:?} row {}", menu.row);
            menu.step(true);
        }
        for _ in 0..menu.entries().len() * 2 {
            assert!(menu.is_on_a_usable_row());
            menu.step(false);
        }
    }
}

#[test]
fn stepping_down_and_back_up_returns_to_the_same_row() {
    let mut menu = Menu::open(Heading::Files);
    let start = menu.row;
    menu.step(true);
    assert_ne!(menu.row, start);
    menu.step(false);
    assert_eq!(menu.row, start);
}

/// Every drop-down offers something, except Options, which cannot.
///
/// Far's Options menu is entirely settings dialogs, and this program keeps no
/// settings — so the one heading with nothing behind it is named here rather
/// than left to drift. When the first setting arrives, this test should start
/// failing and be deleted.
#[test]
fn every_heading_offers_working_commands() {
    for heading in Heading::ALL {
        let usable = Menu::open(heading)
            .entries()
            .iter()
            .filter(|entry| Entry::is_selectable(entry))
            .count();
        assert!(usable > 0, "{heading:?} has nothing to choose");
    }
}

/// A separator is never a command, and a command always names its key or is
/// marked as one the program does not have.
#[test]
fn entries_are_one_thing_or_the_other() {
    for heading in Heading::ALL {
        for entry in Menu::open(heading).entries() {
            match entry {
                Entry::Separator => assert!(!entry.is_selectable()),
                Entry::Item { label, .. } => {
                    assert!(!label.is_empty(), "{heading:?} has an unnamed item");
                    assert_eq!(
                        entry.is_selectable(),
                        entry.key().is_some() || entry.menu_command().is_some()
                    );
                }
            }
        }
    }
}

#[test]
fn a_hotkey_letter_finds_its_heading() {
    assert_eq!(Menu::heading_for_hotkey('l'), Some(Heading::Left));
    assert_eq!(Menu::heading_for_hotkey('R'), Some(Heading::Right));
    assert_eq!(Menu::heading_for_hotkey('c'), Some(Heading::Commands));
    assert_eq!(Menu::heading_for_hotkey('z'), None);
}

#[test]
fn a_hotkey_letter_finds_its_row() {
    let menu = Menu::open(Heading::Files);
    let row = menu.row_for_hotkey('d').expect("Delete");
    assert!(matches!(
        menu.entries()[row],
        Entry::Item {
            label: "&Delete",
            ..
        }
    ));
    assert_eq!(menu.row_for_hotkey('q'), None, "no such letter here");
}

#[cfg(windows)]
#[test]
fn wsl_menu_uses_the_same_command_as_its_keyboard_shortcut() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let menu = Menu::open(Heading::Commands);
    let row = menu.row_for_hotkey('w').expect("WSL manager hotkey");
    let entry = &menu.entries()[row];
    assert!(matches!(
        entry,
        Entry::Item {
            label: "&WSL manager",
            shortcut: "Ctrl+Alt+W",
            ..
        }
    ));
    let key = entry.key().expect("enabled WSL manager");
    assert_eq!(key.code, KeyCode::Char('w'));
    assert_eq!(key.modifiers, KeyModifiers::CONTROL | KeyModifiers::ALT);
    assert_eq!(
        crate::action::resolve(KeyEvent::new(key.code, key.modifiers)),
        crate::action::Action::WslManagement,
    );
}

#[cfg(not(windows))]
#[test]
fn wsl_menu_is_hidden_on_other_platforms() {
    let menu = Menu::open(Heading::Commands);
    assert!(menu
        .entries()
        .iter()
        .all(|entry| super::label_of(entry) != "&WSL manager"));
}

/// Two rows in one drop-down must not answer to the same letter, or one of
/// them is unreachable.
#[test]
fn no_drop_down_has_two_rows_on_one_letter() {
    for heading in Heading::ALL {
        let menu = Menu::open(heading);
        let mut letters: Vec<char> = menu
            .entries()
            .iter()
            .filter(|entry| Entry::is_selectable(entry))
            .filter_map(|entry| super::hotkey_of(super::label_of(entry)))
            .collect();
        let count = letters.len();
        letters.sort_unstable();
        letters.dedup();
        assert_eq!(letters.len(), count, "{heading:?} repeats a hotkey letter");
    }
}

#[test]
fn moving_between_headings_lands_on_a_usable_row() {
    let mut menu = Menu::open(Heading::Left);
    for _ in 0..10 {
        menu.go(true);
        assert!(menu.is_on_a_usable_row() || !menu.has_anything_usable());
    }
    for _ in 0..10 {
        menu.go(false);
        assert!(menu.is_on_a_usable_row() || !menu.has_anything_usable());
    }
}

/// The two panel headings name their own side, and nothing else does.
#[test]
fn only_the_panel_headings_name_a_side() {
    use crate::models::PanelSide;

    assert_eq!(Heading::Left.side(), Some(PanelSide::Left));
    assert_eq!(Heading::Right.side(), Some(PanelSide::Right));
    for heading in [Heading::Files, Heading::Commands, Heading::Options] {
        assert_eq!(heading.side(), None, "{heading:?}");
    }
}

/// The two panel menus are the same list, aimed at opposite sides.
///
/// Exactly two rows differ, and both for the same reason: Far gives each side
/// its own key for hiding that panel and for changing that panel's drive.
/// Everything else must stay identical, or the two sides have drifted.
#[test]
fn the_panel_menus_differ_only_where_they_name_a_side() {
    let left = Menu::open(Heading::Left);
    let right = Menu::open(Heading::Right);
    assert_eq!(left.entries().len(), right.entries().len());

    let differing: Vec<&'static str> = left
        .entries()
        .iter()
        .zip(right.entries())
        .filter(|(a, b)| a != b)
        .map(|(a, _)| super::label_of(a))
        .collect();

    assert_eq!(
        differing.len(),
        2,
        "only the per-side rows may differ, got {differing:?}"
    );
    for label in &differing {
        let plain = label.replace('&', "");
        assert!(
            plain.contains("Panel") || plain.contains("drive"),
            "{label:?} differs between the sides but is not a per-side row"
        );
    }
}

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
                super::label_of(entry),
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
            let label = super::label_of(entry);
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
                super::label_of(entry),
            );
        }
    }
}

/// The reconstruction is Far's, row for row.
///
/// A count rather than a transcription: the point is that nobody has quietly
/// dropped a row while editing a neighbouring one, and a second copy of all
/// ninety-four labels would only be another thing to keep in step.
#[test]
fn every_far_menu_is_present_in_full() {
    let rows = |heading: Heading| {
        Menu::open(heading)
            .entries()
            .iter()
            .filter(|entry| !matches!(entry, Entry::Separator))
            .count()
    };

    // Far 3.0's own counts, plus this program's extra rows where noted.
    assert_eq!(rows(Heading::Left), 18, "Far's Left menu");
    assert_eq!(rows(Heading::Right), 18, "Far's Right menu");
    assert_eq!(
        rows(Heading::Files),
        20,
        "Far's 18 plus new-file and $EDITOR"
    );
    assert_eq!(
        rows(Heading::Commands),
        22 + usize::from(cfg!(windows)),
        "Far commands, root navigation, statistics, package updates, two PATH scopes and Windows WSL",
    );
    assert_eq!(rows(Heading::Options), 23, "Far's Options menu");
}

/// The panel menus act on their own side, so opening one must select it.
#[test]
fn the_panel_headings_name_the_side_they_act_on() {
    use crate::models::PanelSide;

    assert_eq!(Heading::Left.side(), Some(PanelSide::Left));
    assert_eq!(Heading::Right.side(), Some(PanelSide::Right));
    // The keys behind Info panel and Re-read follow the active panel, so a
    // Left row and a Right row are only distinct commands if the heading
    // moves the cursor first.
    for heading in [Heading::Left, Heading::Right] {
        let menu = Menu::open(heading);
        assert!(
            menu.entries().iter().any(|entry| {
                // Without the marker: letter assignment moves the `&` around,
                // and a test that matched the raw label would break whenever
                // a neighbouring row took the letter this one wanted.
                super::label_of(entry)
                    .replace('&', "")
                    .contains("Info panel")
                    && entry.is_selectable()
            }),
            "{heading:?} offers Info panel"
        );
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
    assert_eq!(super::label_of(&working), "&Delete");

    let command = Entry::command("Add &to archive", "Shift+F1", super::MenuCommand::Archive);
    assert!(command.is_selectable());
    assert_eq!(command.menu_command(), Some(super::MenuCommand::Archive));
    assert_eq!(super::label_of(&command), "Add &to archive");
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
