//! Unit tests: the tree panel opens on the panel's own folder, lists real
//! subfolders, follows the cursor into the other panel and turns back into
//! a file panel where the cursor is.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::{enter, expand_or_descend, follow_cursor, refresh, subfolders, toggle, tree_mut};
use crate::models::{App, FolderTree, Panel, PanelKind, PanelSide};

/// `<root>/alpha/inner`, `<root>/Beta`, `<root>/gamma`, a file, and a
/// symlink to `alpha` that must not appear as a folder of its own.
fn tree_root() -> TempDir {
    let root = TempDir::new().expect("temp dir");
    fs::create_dir_all(root.path().join("alpha/inner")).expect("alpha/inner");
    fs::create_dir(root.path().join("Beta")).expect("Beta");
    fs::create_dir(root.path().join("gamma")).expect("gamma");
    fs::write(root.path().join("notes.txt"), "text").expect("file");
    #[cfg(unix)]
    std::os::unix::fs::symlink(root.path().join("alpha"), root.path().join("link"))
        .expect("symlink");
    root
}

fn app_at(left: &Path, right: &Path) -> App {
    App::with_panels(
        Panel::new(left.to_path_buf()).expect("left"),
        Panel::new(right.to_path_buf()).expect("right"),
    )
}

/// A tree rooted at the temp folder itself rather than at `/`, so the rows
/// are the fixture's and not whatever else `/tmp` holds today.
fn open_tree_at(app: &mut App, root: &Path) {
    app.left_panel.kind = PanelKind::Tree(Box::new(FolderTree::rooted_at(root.to_path_buf())));
}

fn names(dirs: &[std::path::PathBuf]) -> Vec<String> {
    dirs.iter()
        .map(|dir| {
            dir.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

#[test]
fn subfolders_are_the_directories_only_in_case_blind_name_order() {
    let root = tree_root();
    assert_eq!(
        names(&subfolders(root.path())),
        vec!["alpha", "Beta", "gamma"]
    );
    assert!(subfolders(&root.path().join("gamma")).is_empty());
    assert!(
        subfolders(&root.path().join("missing")).is_empty(),
        "an unreadable folder lists as empty rather than failing the tree"
    );
}

#[test]
fn toggle_opens_the_tree_on_the_panels_folder_and_closes_it_again() {
    let root = tree_root();
    let inner = root
        .path()
        .join("alpha")
        .join("inner")
        .canonicalize()
        .unwrap();
    let mut app = app_at(&inner, root.path());
    toggle(&mut app);
    let tree = tree_mut(&mut app).expect("a tree");
    assert_eq!(
        tree.root,
        inner.ancestors().last().unwrap(),
        "rooted at the filesystem root"
    );
    assert_eq!(
        tree.selected(),
        Some(inner.as_path()),
        "cursor on the panel's folder"
    );
    assert!(
        tree.rows().len() > 3,
        "every ancestor is open: {:?}",
        tree.rows()
    );
    assert!(app.status.to_string().contains("Tree panel on"));
    toggle(&mut app);
    assert_eq!(app.left_panel.kind, PanelKind::Files);
    assert!(app.status.to_string().contains("Tree panel off"));
    assert!(tree_mut(&mut app).is_none());
}

#[test]
fn expand_lists_a_real_folder_and_a_second_right_steps_into_it() {
    let root = tree_root();
    let mut app = app_at(root.path(), root.path());
    app.config.tree.auto_change_folder = false;
    open_tree_at(&mut app, root.path());
    let cursor_before = tree_mut(&mut app).expect("tree").cursor;
    assert!(
        expand_or_descend(&mut app),
        "the folder has children to show"
    );
    let tree = tree_mut(&mut app).expect("tree");
    assert_eq!(
        tree.cursor, cursor_before,
        "expanding does not move the cursor"
    );
    let shown: Vec<String> = tree
        .rows()
        .iter()
        .map(crate::models::TreeRow::name)
        .collect();
    assert!(
        shown.contains(&"alpha".to_owned()) && shown.contains(&"gamma".to_owned()),
        "{shown:?}"
    );
}

#[test]
fn right_on_an_open_folder_steps_to_its_first_child() {
    let root = tree_root();
    let mut app = app_at(root.path(), root.path());
    app.config.tree.auto_change_folder = false;
    open_tree_at(&mut app, root.path());
    expand_or_descend(&mut app);
    assert!(
        expand_or_descend(&mut app),
        "already open: step to the first child"
    );
    assert_eq!(
        tree_mut(&mut app).expect("tree").selected(),
        Some(root.path().join("alpha").as_path())
    );
}

#[test]
fn right_on_a_file_panel_has_nothing_to_expand() {
    let root = tree_root();
    let mut app = app_at(root.path(), root.path());
    assert!(!expand_or_descend(&mut app));
    assert_eq!(app.left_panel.kind, PanelKind::Files);
}

#[test]
fn follow_cursor_sends_the_other_panel_to_the_folder_under_the_cursor() {
    let root = tree_root();
    let mut app = app_at(root.path(), root.path());
    open_tree_at(&mut app, root.path());
    expand_or_descend(&mut app);
    tree_mut(&mut app).expect("tree").down();
    follow_cursor(&mut app);
    assert_eq!(app.right_panel.path, root.path().join("alpha"));
    assert_eq!(
        app.left_panel.path,
        root.path(),
        "the tree panel's own path is untouched"
    );

    app.config.tree.auto_change_folder = false;
    tree_mut(&mut app).expect("tree").down();
    follow_cursor(&mut app);
    assert_eq!(
        app.right_panel.path,
        root.path().join("alpha"),
        "off: the other panel stays"
    );
}

#[test]
fn follow_cursor_leaves_a_panel_that_is_not_a_file_list_alone() {
    let root = tree_root();
    let mut app = app_at(root.path(), root.path());
    open_tree_at(&mut app, root.path());
    app.active_panel = PanelSide::Right;
    toggle(&mut app);
    app.active_panel = PanelSide::Left;
    expand_or_descend(&mut app);
    tree_mut(&mut app).expect("tree").down();
    follow_cursor(&mut app);
    assert!(app.right_panel.kind.is_tree(), "still a tree");
    assert_eq!(app.right_panel.path, root.path());
}

#[test]
fn enter_turns_the_panel_back_into_a_file_list_at_the_cursor() {
    let root = tree_root();
    let other = TempDir::new().expect("other");
    let mut app = app_at(root.path(), other.path());
    open_tree_at(&mut app, root.path());
    expand_or_descend(&mut app);
    assert!(tree_mut(&mut app).expect("tree").find_first("gamma"));
    enter(&mut app);
    assert_eq!(app.left_panel.kind, PanelKind::Files);
    assert_eq!(app.left_panel.path, root.path().join("gamma"));
    assert_eq!(
        app.right_panel.path,
        root.path().join("gamma"),
        "auto change followed the cursor"
    );
    assert!(
        app.left_panel.get_selected_entry().is_some(),
        "a cursor in the new listing"
    );
}

#[test]
fn enter_on_a_folder_that_vanished_reports_and_keeps_the_old_listing() {
    let root = tree_root();
    let mut app = app_at(root.path(), root.path());
    app.config.tree.auto_change_folder = false;
    open_tree_at(&mut app, root.path());
    expand_or_descend(&mut app);
    assert!(tree_mut(&mut app).expect("tree").find_first("gamma"));
    fs::remove_dir(root.path().join("gamma")).expect("remove gamma");
    enter(&mut app);
    assert_eq!(
        app.left_panel.kind,
        PanelKind::Files,
        "a file panel again all the same"
    );
    assert_eq!(app.left_panel.path, root.path());
    assert!(
        !app.status.to_string().is_empty(),
        "the failure is reported"
    );
}

#[test]
fn refresh_relists_after_the_disk_changed() {
    let root = tree_root();
    let mut app = app_at(root.path(), root.path());
    app.config.tree.auto_change_folder = false;
    open_tree_at(&mut app, root.path());
    expand_or_descend(&mut app);
    fs::create_dir(root.path().join("delta")).expect("delta");
    refresh(&mut app);
    let tree = tree_mut(&mut app).expect("tree");
    let shown: Vec<String> = tree
        .rows()
        .iter()
        .map(crate::models::TreeRow::name)
        .collect();
    assert!(shown.contains(&"delta".to_owned()), "{shown:?}");
    assert!(app.status.to_string().contains("Tree refreshed"));
}

#[test]
fn tree_operations_are_idle_without_a_tree_panel() {
    let root = tree_root();
    let mut app = app_at(root.path(), root.path());
    assert!(!expand_or_descend(&mut app));
    refresh(&mut app);
    enter(&mut app);
    follow_cursor(&mut app);
    assert_eq!(app.left_panel.kind, PanelKind::Files);
}

#[test]
fn expanding_a_leaf_does_nothing() {
    let root = TempDir::new().expect("empty");
    let mut app = app_at(root.path(), root.path());
    open_tree_at(&mut app, root.path());
    assert!(!expand_or_descend(&mut app), "listing an empty root");
    assert!(!expand_or_descend(&mut app), "a leaf cannot open");
}
