//! Purpose: Far's fast find ends on any key that is not part of it, so a query
//! never survives into another folder or panel.

use super::*;

fn alt(character: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(character), KeyModifiers::ALT)
}

/// Every navigation and marking key closes the search before it acts, as the
/// fast-find box does in Far. The prompt must not stay on screen afterwards.
#[test]
fn every_navigation_and_marking_key_ends_the_quick_search() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["alpha.txt", "beta.txt"]);
    for code in [
        KeyCode::Down,
        KeyCode::Up,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::PageUp,
        KeyCode::PageDown,
        KeyCode::Left,
        KeyCode::Right,
        KeyCode::Tab,
        KeyCode::Insert,
    ] {
        dispatch_key(&mut terminal, &mut app, alt('a'));
        assert_eq!(
            app.quick_search.query(),
            Some("a"),
            "{code:?}: search started"
        );
        press(&mut app, &mut terminal, code);
        assert_eq!(app.quick_search.query(), None, "{code:?} kept the search");
    }
}

/// Enter into a folder must not carry the query into it: the next Alt+letter
/// starts a new search there instead of extending a stale one.
#[test]
fn enter_into_a_folder_starts_the_next_search_afresh() {
    let (mut app, mut terminal, dir) = app_and_terminal(&["bravo.txt"]);
    fs::create_dir(dir.path().join("docs")).expect("test setup");
    fs::write(dir.path().join("docs").join("big.log"), b"x").expect("test setup");
    app.left_panel.load_entries().expect("reload");

    dispatch_key(&mut terminal, &mut app, alt('d'));
    press(&mut app, &mut terminal, KeyCode::Enter);
    assert_eq!(app.left_panel.path, dir.path().join("docs"));
    assert_eq!(app.quick_search.query(), None, "Enter kept the search");

    dispatch_key(&mut terminal, &mut app, alt('b'));
    assert_eq!(app.quick_search.query(), Some("b"));
}

/// A letter typed in one panel must not steer a jump in the other: after Tab
/// the search starts from the new panel's first character.
#[test]
fn tab_to_the_other_panel_does_not_extend_the_old_query() {
    let (mut app, mut terminal, dir) = app_and_terminal(&["src.txt", "readme.txt"]);
    app.right_panel = Panel::new(dir.path().to_path_buf()).expect("test setup");

    dispatch_key(&mut terminal, &mut app, alt('s'));
    press(&mut app, &mut terminal, KeyCode::Tab);
    dispatch_key(&mut terminal, &mut app, alt('r'));

    assert_eq!(app.quick_search.query(), Some("r"));
    let panel = app.get_active_panel_ref();
    let selected = panel
        .selected()
        .map(|index| panel.entries[index].name.as_str());
    assert_eq!(selected, Some("readme.txt"));
}

/// Alt+characters and Backspace are the search's own keys: they edit it.
#[test]
fn alt_characters_and_backspace_keep_editing_the_search() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["abc.txt"]);
    dispatch_key(&mut terminal, &mut app, alt('a'));
    dispatch_key(&mut terminal, &mut app, alt('b'));
    assert_eq!(app.quick_search.query(), Some("ab"));
    press(&mut app, &mut terminal, KeyCode::Backspace);
    assert_eq!(app.quick_search.query(), Some("a"));
}
