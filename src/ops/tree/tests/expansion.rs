//! Purpose: Opening the tree and expanding folders in it.

use super::*;

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
    assert_eq!(subfolders(&root.path().join("gamma")).len(), 0);
    assert_eq!(
        subfolders(&root.path().join("missing")).len(),
        0,
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
fn expanding_a_leaf_does_nothing() {
    let root = TempDir::new().expect("empty");
    let mut app = app_at(root.path(), root.path());
    open_tree_at(&mut app, root.path());
    assert!(!expand_or_descend(&mut app), "listing an empty root");
    assert!(!expand_or_descend(&mut app), "a leaf cannot open");
}
