//! Purpose: The Tree panel (`Ctrl+T`) — the active panel becomes a folder
//! tree rooted at its filesystem root, or goes back to a file list.
//!
//! Everything that touches the disk on the tree's behalf is here: listing a
//! folder's subfolders when a row is opened, sending the other panel to the
//! folder under the cursor, and turning the tree back into a file panel. The
//! tree itself (`models::tree`) only ever sees the lists this module hands it.

use std::fs;
use std::path::{Path, PathBuf};

use crate::models::{App, Expansion, FolderTree, PanelKind};
use crate::status::Status;

/// `Ctrl+T`: the tree in place of the file list, or the file list back.
///
/// The tree opens with every folder on the way to the panel's directory
/// already expanded and the cursor on that directory — Far's tree panel
/// opens where the user is, not at the root with everything folded.
pub fn toggle(app: &mut App) {
    let panel = app.get_active_panel();
    if panel.kind.is_tree() {
        panel.kind = PanelKind::Files;
        app.set_status(Status::CommandOutput("Tree panel off".to_owned()));
        return;
    }
    // Directory listings return long, consistently cased names on Windows.
    // Resolve short names and aliases before comparing the ancestor paths.
    let path = fs::canonicalize(&panel.path).unwrap_or_else(|_| panel.path.clone());
    let mut tree = FolderTree::rooted_at(filesystem_root(&path));
    tree.reveal(&path, &mut subfolders);
    panel.kind = PanelKind::Tree(Box::new(tree));
    app.set_status(Status::CommandOutput("Tree panel on".to_owned()));
}

/// `/` on Unix; the drive (`C:\`) on Windows.
fn filesystem_root(path: &Path) -> PathBuf {
    path.ancestors()
        .last()
        .map_or_else(|| path.to_path_buf(), Path::to_path_buf)
}

/// The tree the active panel shows, if it shows one.
pub fn tree_mut(app: &mut App) -> Option<&mut FolderTree> {
    match &mut app.get_active_panel().kind {
        PanelKind::Tree(tree) => Some(tree),
        PanelKind::Files | PanelKind::QuickView(_) => None,
    }
}

/// The subfolders of `dir`, in case-blind name order — the one listing the
/// tree is built from.
///
/// Symbolic links are left out even when they point at folders: a link back
/// up the tree would make it endless, and Far's tree is the disk's own
/// structure. An unreadable folder lists as empty rather than as an error —
/// the tree panel is for looking around, and a permission-denied on one
/// branch is not a reason to refuse the rest.
pub fn subfolders(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut folders: Vec<PathBuf> = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path())
        .collect();
    folders.sort_by_cached_key(|folder| {
        folder
            .file_name()
            .map(|name| name.to_string_lossy().to_lowercase())
    });
    folders
}

/// `Right`: open the folder under the cursor, or step into it when it is
/// already open. `false` when the key had nothing to do — a leaf, or no tree.
pub fn expand_or_descend(app: &mut App) -> bool {
    let Some(tree) = tree_mut(app) else {
        return false;
    };
    let acted = match tree.selected_row().map(|row| row.expansion) {
        Some(Expansion::Collapsed) => tree.expand(&mut subfolders),
        Some(Expansion::Expanded) => tree.move_to_first_child(),
        Some(Expansion::Leaf) | None => false,
    };
    if acted {
        follow_cursor(app);
    }
    acted
}

/// `Ctrl+R`: list the open folders again after the disk changed behind them.
pub fn refresh(app: &mut App) {
    let Some(tree) = tree_mut(app) else {
        return;
    };
    tree.refresh(&mut subfolders);
    app.set_status(Status::CommandOutput("Tree refreshed".to_owned()));
}

/// The folder under the tree's cursor, owned so the tree borrow ends before
/// the other panel is touched.
fn selected_folder(app: &mut App) -> Option<PathBuf> {
    tree_mut(app)?.selected().map(Path::to_path_buf)
}

/// Far's "auto change folder": the other panel shows the folder under the
/// tree's cursor.
///
/// Only a file panel follows — a second tree or a quick view has its own idea
/// of what to show — and a folder it cannot read is reported without moving.
pub fn follow_cursor(app: &mut App) {
    if !app.config.tree.auto_change_folder {
        return;
    }
    let Some(target) = selected_folder(app) else {
        return;
    };
    let other = app.get_inactive_panel();
    if other.kind != PanelKind::Files || other.path == target {
        return;
    }
    if let Err(error) = other.navigate_to(target) {
        app.set_status(Status::Error(format!("Cannot open folder: {error}")));
    }
}

/// `Enter`: the panel is a file list again, at the folder under the cursor.
///
/// When that folder cannot be read the panel still leaves the tree — the user
/// asked for the file list back — and keeps its last listing.
pub fn enter(app: &mut App) {
    let Some(target) = selected_folder(app) else {
        return;
    };
    follow_cursor(app);
    let panel = app.get_active_panel();
    panel.kind = PanelKind::Files;
    match panel.navigate_to(target) {
        Ok(()) => {
            panel.select(Some(0));
            app.set_status(Status::Cleared);
        }
        Err(error) => app.set_status(Status::Error(format!("Cannot open folder: {error}"))),
    }
}

#[cfg(test)]
mod tests;
