//! Purpose: Unit tests for `FolderTree::rebuild` when the root path is empty.

use super::FolderTree;

#[test]
fn rebuild_on_empty_root_tree_keeps_rows_empty() {
    let mut tree = FolderTree::default();
    tree.refresh(&mut |_| Vec::new());
    assert_eq!(tree.rows(), []);
    assert_eq!(tree.selected(), None);
}
