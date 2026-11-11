//! Purpose: Quick-search keys of the main view — Alt+character starts or
//! extends the search, Backspace edits it, and Esc/Delete clear it.
use super::*;

#[test]
fn test_resolve_quick_search() {
    assert_eq!(
        resolve(key_mod(KeyCode::Char('a'), KeyModifiers::ALT)),
        Action::QuickSearch('a')
    );
    assert_eq!(
        resolve(key(KeyCode::Backspace)),
        Action::QuickSearchBackspace
    );
    assert_eq!(resolve(key(KeyCode::Esc)), Action::QuickSearchClear);
    // A printable key no longer clears the search — it starts a command.
    assert_eq!(
        resolve(key(KeyCode::Char('z'))),
        Action::CommandLineChar('z')
    );
    assert_eq!(resolve(key(KeyCode::Delete)), Action::QuickSearchClear);
}

#[test]
fn test_resolve_quick_search_accepts_any_unclaimed_alt_character() {
    for character in ['A', '7', '?', 'λ'] {
        assert_eq!(
            resolve(key_mod(KeyCode::Char(character), KeyModifiers::ALT)),
            Action::QuickSearch(character)
        );
    }
}

#[test]
fn test_alt_q_is_quick_search_not_quit() {
    // `q` no longer quits at all, so Alt+q is nothing but quick search.
    assert_eq!(
        resolve(key_mod(KeyCode::Char('q'), KeyModifiers::ALT)),
        Action::QuickSearch('q')
    );
}

/// Alt+'-', Alt+'+' and Alt+'*' extend the quick search like any other
/// Alt+character, so a name starting with one of them can be found. Only the
/// bare keys (Far's Gray -, Gray +, Gray *) open the mask dialogs.
#[test]
fn alt_minus_plus_and_star_quick_search_instead_of_opening_the_mask_dialogs() {
    for character in ['-', '+', '*'] {
        assert_eq!(
            resolve(key_mod(KeyCode::Char(character), KeyModifiers::ALT)),
            Action::QuickSearch(character),
            "Alt+{character}"
        );
    }
    assert_eq!(resolve(key(KeyCode::Char('+'))), Action::SelectGroup);
    assert_eq!(resolve(key(KeyCode::Char('-'))), Action::UnselectGroup);
    assert_eq!(resolve(key(KeyCode::Char('*'))), Action::InvertSelection);
}

/// Only Alt+character and Backspace belong to the search; everything else,
/// navigation included, ends it.
#[test]
fn only_alt_characters_and_backspace_edit_the_quick_search() {
    assert!(Action::QuickSearch('a').edits_quick_search());
    assert!(Action::QuickSearchBackspace.edits_quick_search());
    for other in [
        Action::NavDown,
        Action::Open,
        Action::SwitchPanel,
        Action::ToggleMark,
        Action::QuickSearchClear,
        Action::CommandLineChar('a'),
    ] {
        assert!(!other.edits_quick_search(), "{other:?}");
    }
}
