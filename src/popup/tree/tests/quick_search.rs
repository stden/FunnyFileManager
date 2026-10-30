//! Purpose: the tree panel's quick search — typing searches folder names, Backspace shortens it, Esc ends it, and Ctrl+Enter moves to the next match.

use super::*;

#[test]
fn typing_quick_searches_folder_names_and_restarts_when_nothing_matches() {
    let (root, mut app) = tree_app();
    assert!(process(&mut app, key(KeyCode::Char('g'))));
    assert_eq!(
        selected(&mut app),
        root.path().join("gamma").display().to_string()
    );
    assert_eq!(tree_mut(&mut app).expect("tree").search.query(), Some("g"));
    assert!(
        process(&mut app, alt(KeyCode::Char('b'))),
        "Alt+letter is Far's fast find"
    );
    assert_eq!(
        tree_mut(&mut app).expect("tree").search.query(),
        Some("b"),
        "'gb' matches nothing, so the search restarts from the new letter"
    );
    assert_eq!(
        selected(&mut app),
        root.path().join("beta").display().to_string()
    );
}

#[test]
fn backspace_shortens_the_search_and_esc_ends_it() {
    let (_root, mut app) = tree_app();
    process(&mut app, key(KeyCode::Char('b')));
    assert!(process(&mut app, key(KeyCode::Backspace)));
    assert!(tree_mut(&mut app).expect("tree").search.query().is_none());
    assert!(
        !process(&mut app, key(KeyCode::Backspace)),
        "nothing left to shorten"
    );
    process(&mut app, key(KeyCode::Char('b')));
    assert!(process(&mut app, key(KeyCode::Esc)));
    assert!(tree_mut(&mut app).expect("tree").search.query().is_none());
    assert!(
        !process(&mut app, key(KeyCode::Esc)),
        "no search: Esc is not the tree's"
    );
}

#[test]
fn ctrl_enter_moves_to_the_next_match() {
    let (root, mut app) = tree_app();
    fs::create_dir(root.path().join("alpine")).expect("alpine");
    process(&mut app, ctrl(KeyCode::Char('r')));
    assert!(app.status.to_string().contains("Tree refreshed"));
    process(&mut app, key(KeyCode::Char('a')));
    assert_eq!(
        selected(&mut app),
        root.path().join("alpha").display().to_string()
    );
    assert!(process(&mut app, ctrl(KeyCode::Enter)));
    assert_eq!(
        selected(&mut app),
        root.path().join("alpine").display().to_string()
    );
    assert!(process(&mut app, ctrl(KeyCode::Enter)), "wraps");
    assert_eq!(
        selected(&mut app),
        root.path().join("alpha").display().to_string()
    );
}
