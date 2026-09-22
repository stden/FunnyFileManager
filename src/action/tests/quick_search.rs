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
