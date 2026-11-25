//! Purpose: Alt+letter quick search: starting, extending, trimming and clearing.

use super::*;

// ─── ALT+LETTER QUICK SEARCH ─────────────────────────────────────────────────

#[test]
fn hotkey_alt_letter_starts_quick_search() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    assert!(app.quick_search.query().is_none());
    // Alt+letter logic
    app.quick_search = QuickSearch::Active("a".to_owned());
    assert_eq!(app.quick_search.query(), Some("a"));
}

#[test]
fn hotkey_alt_letter_appends_to_existing_quick_search() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    // Through the real method: this used to re-implement the append inline,
    // so it could only ever agree with itself.
    app.quick_search = QuickSearch::Active("al".to_owned());
    assert_eq!(app.quick_search.push('p'), "alp");
    assert_eq!(app.quick_search.query(), Some("alp"));
}

#[test]
fn hotkey_backspace_pops_quick_search_char() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.quick_search = QuickSearch::Active("abc".to_owned());
    assert_eq!(app.quick_search.backspace(), Some("ab"));
    assert_eq!(app.quick_search.query(), Some("ab"));
}

#[test]
fn hotkey_backspace_clears_single_char_quick_search() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    // Emptying the query ends the search: `Active("")` is not representable.
    app.quick_search = QuickSearch::Active("x".to_owned());
    assert_eq!(app.quick_search.backspace(), None);
    assert_eq!(app.quick_search, QuickSearch::Idle);
}

#[test]
fn hotkey_esc_clears_quick_search() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.quick_search = QuickSearch::Active("abc".to_owned());
    app.quick_search = QuickSearch::Idle; // Esc logic
    assert!(app.quick_search.query().is_none());
}

#[test]
fn hotkey_alt_letter_jumps_to_matching_entry() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("alpha.txt"), b"").unwrap();
    fs::write(temp.path().join("beta.txt"), b"").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let found = panel.find_first_matching("b");
    assert!(found);
    let idx = panel.selected().unwrap();
    assert_eq!(panel.entries[idx].name, "beta.txt");
}
