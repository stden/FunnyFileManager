//! Folder tree (`Ctrl+T`) acceptance steps: toggle, expand/climb, passive-panel
//! auto-follow, Enter navigation, typed folder search, and F5–F8 guidance.

use super::{BehaviorWorld, Case};
use crate::models::PanelKind;
use cucumber::{given, then};

#[given(expr = "a left folder hierarchy with {string} and {string}")]
fn left_folder_hierarchy(world: &mut BehaviorWorld, first: String, second: String) {
    let case = world.case();
    let left = case.root.path().join("left");
    for rel in [&first, &second] {
        let dir = left.join(rel);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("marker.txt"), rel).unwrap();
    }
    case.app.left_panel.load_entries().unwrap();
    case.paint();
}

#[then("the left panel is a folder tree")]
fn left_is_tree(world: &mut BehaviorWorld) {
    assert!(
        world.case().app.left_panel.kind.is_tree(),
        "expected left panel to be in Tree mode"
    );
}

#[then("the left panel is a file list")]
fn left_is_files(world: &mut BehaviorWorld) {
    assert_eq!(world.case().app.left_panel.kind, PanelKind::Files);
}

#[then(expr = "the tree cursor selects folder {string}")]
fn tree_cursor_selects(world: &mut BehaviorWorld, expected: String) {
    let PanelKind::Tree(tree) = &world.case().app.left_panel.kind else {
        panic!("expected left panel to be a folder tree");
    };
    let row = tree.selected_row().expect("tree must have a selected row");
    assert_eq!(row.name(), expected);
}

#[then(expr = "the {word} panel is at left subfolder {string}")]
fn panel_at_left_subfolder(world: &mut BehaviorWorld, side: String, subfolder: String) {
    let case = world.case();
    let expected = std::fs::canonicalize(case.root.path().join("left").join(subfolder)).unwrap();
    let actual = std::fs::canonicalize(&case.app.panel(Case::side(&side)).path).unwrap();
    assert_eq!(actual, expected);
}
