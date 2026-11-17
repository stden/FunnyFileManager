//! Unit tests: the folder tree expands one level at a time through an injected
//! lister, keeps its rows depth-first with Far's guide lines, and moves the
//! cursor the way the tree panel and the Find folder dialog both need.

use std::path::{Path, PathBuf};

use super::{Branch, Expansion, FolderTree, Guide, TreeRow};

/// A fixed tree, so no test reads the disk:
///
/// ```text
/// /
/// ├─etc
/// │ └─ssh
/// ├─home
/// │ ├─ann
/// │ └─bob
/// └─var
/// ```
fn lister(dir: &Path) -> Vec<PathBuf> {
    let children: &[&str] = match dir.to_str().unwrap_or_default() {
        "/" => &["/etc", "/home", "/var"],
        "/etc" => &["/etc/ssh"],
        "/home" => &["/home/ann", "/home/bob"],
        _ => &[],
    };
    children.iter().map(PathBuf::from).collect()
}

fn root() -> FolderTree {
    FolderTree::rooted_at(PathBuf::from("/"))
}

fn names(tree: &FolderTree) -> Vec<String> {
    tree.rows().iter().map(TreeRow::name).collect()
}

mod tests_cursor;
mod tests_shape;
