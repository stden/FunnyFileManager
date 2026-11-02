//! Purpose: The other panel follows the tree cursor; Enter, refresh and no tree at all.

use super::*;

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
    assert_ne!(app.status.to_string().len(), 0, "the failure is reported");
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
