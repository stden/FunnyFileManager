//! Purpose: hotkey letters: every heading and row answers to one, and no two share it.
use super::*;

#[test]
fn every_heading_has_a_hotkey_and_they_are_all_different() {
    let mut letters: Vec<char> = Heading::ALL
        .into_iter()
        .map(|heading| {
            hotkey_of(heading.label()).unwrap_or_else(|| panic!("{heading:?} has no hotkey"))
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
            .filter_map(|entry| hotkey_of(label_of(entry)))
            .collect();
        let count = letters.len();
        letters.sort_unstable();
        letters.dedup();
        assert_eq!(letters.len(), count, "{heading:?} repeats a hotkey letter");
    }
}
