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

mod expansion;
mod following;
