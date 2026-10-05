//! Unit tests: the tree panel claims the keys that move, open and search its
//! rows, and leaves every other key to the file-panel actions.

use std::fs;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

use super::process;
use crate::models::{App, FolderTree, Panel, PanelKind};
use crate::ops::tree::tree_mut;
use crate::popup::tests::key;

mod quick_search;

fn ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}

fn alt(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::ALT)
}

/// `<root>/alpha/inner`, `<root>/beta`, `<root>/gamma`; the tree panel is
/// the left one, rooted at `<root>` (not `/`: the rows above a temp folder
/// are whatever else is in `/tmp`) with its children shown.
fn tree_app() -> (TempDir, App) {
    let root = TempDir::new().expect("temp dir");
    fs::create_dir_all(root.path().join("alpha/inner")).expect("alpha/inner");
    fs::create_dir(root.path().join("beta")).expect("beta");
    fs::create_dir(root.path().join("gamma")).expect("gamma");
    let mut app = App::with_panels(
        Panel::new(root.path().to_path_buf()).expect("left"),
        Panel::new(root.path().to_path_buf()).expect("right"),
    );
    app.left_panel.kind =
        PanelKind::Tree(Box::new(FolderTree::rooted_at(root.path().to_path_buf())));
    assert!(
        process(&mut app, key(KeyCode::Right)),
        "open the root's children"
    );
    (root, app)
}

fn selected(app: &mut App) -> String {
    tree_mut(app)
        .and_then(|tree| tree.selected())
        .map(|path| path.display().to_string())
        .unwrap_or_default()
}

#[test]
fn a_file_panel_is_not_asked() {
    let mut app = App::new().expect("test setup");
    assert!(!process(&mut app, key(KeyCode::Down)));
}

#[test]
fn arrows_move_the_cursor_and_the_other_panel_follows() {
    let (root, mut app) = tree_app();
    assert!(process(&mut app, key(KeyCode::Down)));
    assert_eq!(
        selected(&mut app),
        root.path().join("alpha").display().to_string()
    );
    assert_eq!(
        app.right_panel.path,
        root.path().join("alpha"),
        "auto change folder"
    );
    assert!(process(&mut app, key(KeyCode::End)));
    assert!(process(&mut app, key(KeyCode::Home)));
    assert_eq!(selected(&mut app), root.path().display().to_string());
    assert!(process(&mut app, key(KeyCode::PageDown)));
    assert!(process(&mut app, key(KeyCode::PageUp)));
    assert_eq!(selected(&mut app), root.path().display().to_string());
    assert!(
        process(&mut app, key(KeyCode::Up)),
        "consumed even at the top"
    );
}

#[test]
fn right_opens_a_folder_and_left_closes_it_or_climbs_out() {
    let (root, mut app) = tree_app();
    process(&mut app, key(KeyCode::Down));
    assert!(process(&mut app, key(KeyCode::Right)));
    let rows = tree_mut(&mut app).expect("tree").rows().len();
    assert!(process(&mut app, key(KeyCode::Right)), "and step into it");
    assert_eq!(
        selected(&mut app),
        root.path()
            .join("alpha")
            .join("inner")
            .display()
            .to_string()
    );
    assert!(
        process(&mut app, key(KeyCode::Left)),
        "a collapsed leaf: climb out"
    );
    assert_eq!(
        selected(&mut app),
        root.path().join("alpha").display().to_string()
    );
    assert!(
        process(&mut app, key(KeyCode::Left)),
        "an open folder: close it"
    );
    assert_eq!(tree_mut(&mut app).expect("tree").rows().len(), rows - 1);
    assert!(
        process(&mut app, key(KeyCode::Right)),
        "a leaf: nothing to open, still the tree's key"
    );
}

#[test]
fn enter_makes_the_panel_a_file_list_where_the_cursor_is() {
    let (root, mut app) = tree_app();
    process(&mut app, key(KeyCode::Down));
    assert!(process(&mut app, key(KeyCode::Enter)));
    assert_eq!(app.left_panel.kind, PanelKind::Files);
    assert_eq!(app.left_panel.path, root.path().join("alpha"));
}

#[test]
fn plus_and_minus_walk_the_siblings_of_one_level() {
    let (root, mut app) = tree_app();
    process(&mut app, key(KeyCode::Down));
    process(&mut app, key(KeyCode::Right));
    assert!(process(&mut app, key(KeyCode::Char('+'))));
    assert_eq!(
        selected(&mut app),
        root.path().join("beta").display().to_string(),
        "past alpha's child"
    );
    assert!(process(&mut app, key(KeyCode::Char('-'))));
    assert_eq!(
        selected(&mut app),
        root.path().join("alpha").display().to_string()
    );
}

#[test]
fn file_operation_keys_are_refused_with_a_word_rather_than_acting_on_the_hidden_list() {
    let (_root, mut app) = tree_app();
    for n in 5..=8 {
        assert!(process(&mut app, key(KeyCode::F(n))));
        assert!(
            app.status.to_string().contains("Tree panel"),
            "{}",
            app.status
        );
    }
}

#[test]
fn the_keys_the_tree_does_not_own_fall_through() {
    let (_root, mut app) = tree_app();
    assert!(
        !process(&mut app, ctrl(KeyCode::Char('t'))),
        "Ctrl+T turns the tree off"
    );
    assert!(!process(&mut app, key(KeyCode::Tab)));
    assert!(
        !process(&mut app, key(KeyCode::Char(':'))),
        "the command line"
    );
    assert!(!process(&mut app, key(KeyCode::F(1))));
    assert!(!process(&mut app, key(KeyCode::F(9))));
    assert!(!process(&mut app, ctrl(KeyCode::Char('o'))));
    assert!(app.left_panel.kind.is_tree());
}

#[test]
fn search_helpers_are_idle_without_a_tree() {
    let mut app = App::new().expect("app");
    assert!(!super::shorten_search(&mut app));
    assert!(!super::clear_search(&mut app));
    let mut tree = FolderTree::default();
    super::step(&mut tree, KeyCode::Left, 1);
}
