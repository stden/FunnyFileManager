//! Purpose: what each drop-down holds: usable rows, entry shape, the WSL row and Far's row counts.
use super::*;

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
                    assert_ne!(label.len(), 0, "{heading:?} has an unnamed item");
                    assert_eq!(entry.is_selectable(), entry.action().is_some());
                }
            }
        }
    }
}

#[cfg(windows)]
#[test]
fn wsl_menu_uses_the_same_command_as_its_keyboard_shortcut() {
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
    assert_eq!(entry.action(), Some(crate::action::Action::WslManagement));
}

#[cfg(not(windows))]
#[test]
fn wsl_menu_is_hidden_on_other_platforms() {
    let menu = Menu::open(Heading::Commands);
    assert!(menu
        .entries()
        .iter()
        .all(|entry| label_of(entry) != "&WSL manager"));
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
        29 + usize::from(cfg!(windows)),
        "Far commands, root navigation, statistics, LSP, package updates, two PATH scopes, \
         bulk Git pulls, temporary-file cleanup, the space analysis, token usage, the Things \
         catalog, the MCP server installer and Windows WSL",
    );
    assert_eq!(rows(Heading::Options), 23, "Far's Options menu");
}
