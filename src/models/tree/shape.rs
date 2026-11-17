//! Purpose: How the tree changes shape — expanding, collapsing, revealing and refreshing folders.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::FolderTree;

impl FolderTree {
    // ── shape ──────────────────────────────────────────────────────

    /// Show the subfolders of the folder under the cursor, listing them the
    /// first time. `true` when the row gained children.
    pub fn expand(&mut self, list: &mut impl FnMut(&Path) -> Vec<PathBuf>) -> bool {
        let Some(path) = self.selected().map(Path::to_path_buf) else {
            return false;
        };
        if self.expanded.contains(&path) {
            return false;
        }
        let opened = self.open(&path, list);
        self.rebuild();
        opened
    }

    /// List `dir` if it has not been, and mark it open when there is
    /// something to show. A childless folder is never "open": `Left` on it
    /// should climb to the parent, not close nothing first.
    fn open(&mut self, dir: &Path, list: &mut impl FnMut(&Path) -> Vec<PathBuf>) -> bool {
        self.load(dir, list);
        let has_children = self.children.get(dir).is_some_and(|kids| !kids.is_empty());
        if has_children {
            self.expanded.insert(dir.to_path_buf());
        }
        has_children
    }

    /// Hide the subfolders of the folder under the cursor. `true` when there
    /// were any to hide.
    pub fn collapse(&mut self) -> bool {
        let Some(path) = self.selected().map(Path::to_path_buf) else {
            return false;
        };
        if !self.expanded.remove(&path) {
            return false;
        }
        self.rebuild();
        true
    }

    /// Open every folder on the way to `path` and put the cursor on it —
    /// what `Ctrl+T` does with the panel's own directory, so the tree opens
    /// where the user is rather than at the root. `false` when the listings
    /// never reach `path`.
    pub fn reveal(&mut self, path: &Path, list: &mut impl FnMut(&Path) -> Vec<PathBuf>) -> bool {
        if !path.starts_with(&self.root) {
            return false;
        }
        let ancestors: Vec<PathBuf> = path
            .ancestors()
            .skip(1)
            .take_while(|dir| dir.starts_with(&self.root))
            .map(Path::to_path_buf)
            .collect();
        for dir in ancestors.into_iter().rev() {
            if !self.open(&dir, list) {
                break;
            }
        }
        self.rebuild();
        let Some(row) = self.rows.iter().position(|row| row.path == path) else {
            return false;
        };
        self.cursor = row;
        true
    }

    /// Forget every listing and list the expanded folders again — `Ctrl+R`
    /// after the disk changed behind the tree. Folders that vanished leave
    /// the expanded set; the cursor stays on its folder if that survived.
    pub fn refresh(&mut self, list: &mut impl FnMut(&Path) -> Vec<PathBuf>) {
        let was_on = self.selected().map(Path::to_path_buf);
        self.children.clear();
        let expanded = std::mem::take(&mut self.expanded);
        let mut listed = HashSet::new();
        for dir in expanded {
            if (dir == self.root || listed.contains(&dir)) && self.open(&dir, list) {
                listed.extend(self.children.get(&dir).into_iter().flatten().cloned());
            }
        }
        self.rebuild();
        if let Some(row) = was_on.and_then(|path| self.rows.iter().position(|row| row.path == path))
        {
            self.cursor = row;
        }
    }

    fn load(&mut self, dir: &Path, list: &mut impl FnMut(&Path) -> Vec<PathBuf>) {
        if !self.children.contains_key(dir) {
            self.children.insert(dir.to_path_buf(), list(dir));
        }
    }
}
