//! Purpose: The folder tree behind the Tree panel (`Ctrl+T`).
//!
//! The tree is expanded lazily, one level at a time, through a lister the
//! caller injects — nothing here reads the disk, so the shape of the tree can
//! be tested against a made-up filesystem and drawn without touching a real
//! one.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::casefold::starts_with_ignore_case;
use crate::quicksearch::QuickSearch;

/// What a row's ancestor draws in its column: a line running on to a later
/// sibling, or nothing because that ancestor was the last of its level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Guide {
    Line,
    Gap,
}

/// How a row hangs off its parent — Far's `├─` and `└─`; the root hangs off
/// nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Branch {
    Root,
    Middle,
    Last,
}

/// Whether a row's subfolders are on screen.
///
/// `Leaf` is a folder that was listed and had none: it looks like `Collapsed`
/// but Right cannot open it, and the key handler must not keep re-listing it
/// hoping for a different answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expansion {
    Collapsed,
    Expanded,
    Leaf,
}

/// One folder on screen: its place in the tree, worked out once when the rows
/// were rebuilt so the renderer and the cursor moves read it rather than
/// derive it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeRow {
    pub path: PathBuf,
    /// One entry per ancestor below the root, outermost first.
    pub guides: Vec<Guide>,
    pub branch: Branch,
    pub expansion: Expansion,
}

impl TreeRow {
    /// How deep the row sits: the root is 0, its children 1.
    #[must_use]
    pub fn depth(&self) -> usize {
        match self.branch {
            Branch::Root => 0,
            Branch::Middle | Branch::Last => self.guides.len() + 1,
        }
    }

    /// The folder's own name; the root has none, so it shows its whole path.
    #[must_use]
    pub fn name(&self) -> String {
        self.path.file_name().map_or_else(
            || self.path.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        )
    }
}

/// A folder tree and where the cursor is in it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FolderTree {
    /// Where the tree is rooted: the filesystem root of the panel's path.
    pub root: PathBuf,
    /// Which row carries the cursor.
    pub cursor: usize,
    /// The first row on screen, kept by the renderer between frames the way
    /// the file panel keeps its list offset.
    pub scroll: usize,
    /// Typed quick search, shown on the panel's info row.
    pub search: QuickSearch,
    /// The folders shown, depth-first, one per row. Rebuilt whenever the
    /// expanded set or the known children change.
    rows: Vec<TreeRow>,
    /// Folders whose children are on screen.
    expanded: BTreeSet<PathBuf>,
    /// Every listing done so far, so collapsing and re-expanding a folder
    /// costs nothing; `Ctrl+R` forgets it.
    children: BTreeMap<PathBuf, Vec<PathBuf>>,
}

impl FolderTree {
    /// A tree rooted at `root`, showing only the root until it is expanded.
    #[must_use]
    pub fn rooted_at(root: PathBuf) -> Self {
        let mut tree = Self {
            root,
            ..Self::default()
        };
        tree.rebuild();
        tree
    }

    #[must_use]
    pub fn rows(&self) -> &[TreeRow] {
        &self.rows
    }

    /// The folder under the cursor, if the tree has any rows.
    #[must_use]
    pub fn selected(&self) -> Option<&Path> {
        self.rows.get(self.cursor).map(|row| row.path.as_path())
    }

    #[must_use]
    pub fn selected_row(&self) -> Option<&TreeRow> {
        self.rows.get(self.cursor)
    }

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
        for dir in expanded {
            if dir == self.root || self.is_listed_child(&dir) {
                self.open(&dir, list);
            }
        }
        self.rebuild();
        if let Some(row) = was_on.and_then(|path| self.rows.iter().position(|row| row.path == path))
        {
            self.cursor = row;
        }
    }

    /// Whether `dir`'s parent has been listed and still names it. The
    /// expanded set is walked in path order, so a parent is always relisted
    /// before its children are asked about.
    fn is_listed_child(&self, dir: &Path) -> bool {
        dir.parent()
            .and_then(|parent| self.children.get(parent))
            .is_some_and(|kids| kids.iter().any(|kid| kid == dir))
    }

    fn load(&mut self, dir: &Path, list: &mut impl FnMut(&Path) -> Vec<PathBuf>) {
        if !self.children.contains_key(dir) {
            self.children.insert(dir.to_path_buf(), list(dir));
        }
    }

    fn expansion_of(&self, path: &Path) -> Expansion {
        match self.children.get(path) {
            Some(kids) if kids.is_empty() => Expansion::Leaf,
            Some(_) if self.expanded.contains(path) => Expansion::Expanded,
            _ => Expansion::Collapsed,
        }
    }

    /// Lay the rows out again from the root. The cursor is clamped, not
    /// reset: a refresh that removed rows below it should not send it home.
    fn rebuild(&mut self) {
        let mut rows = Vec::new();
        if !self.root.as_os_str().is_empty() {
            let root = self.root.clone();
            self.push_rows(&root, &[], Branch::Root, &mut rows);
        }
        self.rows = rows;
        self.cursor = self.cursor.min(self.rows.len().saturating_sub(1));
    }

    fn push_rows(&self, path: &Path, guides: &[Guide], branch: Branch, rows: &mut Vec<TreeRow>) {
        let expansion = self.expansion_of(path);
        rows.push(TreeRow {
            path: path.to_path_buf(),
            guides: guides.to_vec(),
            branch,
            expansion,
        });
        if expansion != Expansion::Expanded {
            return;
        }
        let Some(kids) = self.children.get(path) else {
            return;
        };
        let mut child_guides = guides.to_vec();
        match branch {
            Branch::Root => {}
            Branch::Middle => child_guides.push(Guide::Line),
            Branch::Last => child_guides.push(Guide::Gap),
        }
        let last = kids.len().saturating_sub(1);
        for (index, kid) in kids.iter().enumerate() {
            let child_branch = if index == last {
                Branch::Last
            } else {
                Branch::Middle
            };
            self.push_rows(kid, &child_guides, child_branch, rows);
        }
    }

    // ── cursor ─────────────────────────────────────────────────────

    pub fn up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn down(&mut self) {
        self.cursor = (self.cursor + 1).min(self.rows.len().saturating_sub(1));
    }

    pub fn first(&mut self) {
        self.cursor = 0;
    }

    pub fn last(&mut self) {
        self.cursor = self.rows.len().saturating_sub(1);
    }

    pub fn page_up(&mut self, rows: usize) {
        self.cursor = self.cursor.saturating_sub(rows);
    }

    pub fn page_down(&mut self, rows: usize) {
        self.cursor = self
            .cursor
            .saturating_add(rows)
            .min(self.rows.len().saturating_sub(1));
    }

    /// Up to the row this folder hangs off. `false` on the root.
    pub fn to_parent(&mut self) -> bool {
        let Some(parent) = self
            .selected()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
        else {
            return false;
        };
        self.move_to(|row| row.path == parent)
    }

    /// Down to the first subfolder on screen. `false` unless the row is expanded.
    pub fn to_first_child(&mut self) -> bool {
        let Some(row) = self.selected_row() else {
            return false;
        };
        if row.expansion != Expansion::Expanded {
            return false;
        }
        self.cursor += 1;
        true
    }

    /// Far's `Gray +`: the next folder at the same level under the same
    /// parent, skipping whatever is open in between.
    pub fn next_sibling(&mut self) -> bool {
        let Some((depth, parent)) = self.level() else {
            return false;
        };
        let Some(offset) = self.rows[self.cursor + 1..]
            .iter()
            .position(|row| row.depth() == depth && row.path.parent() == parent.as_deref())
        else {
            return false;
        };
        self.cursor += offset + 1;
        true
    }

    /// Far's `Gray -`: the previous folder at the same level.
    pub fn previous_sibling(&mut self) -> bool {
        let Some((depth, parent)) = self.level() else {
            return false;
        };
        let Some(at) = self.rows[..self.cursor]
            .iter()
            .rposition(|row| row.depth() == depth && row.path.parent() == parent.as_deref())
        else {
            return false;
        };
        self.cursor = at;
        true
    }

    fn level(&self) -> Option<(usize, Option<PathBuf>)> {
        let row = self.selected_row()?;
        Some((row.depth(), row.path.parent().map(Path::to_path_buf)))
    }

    fn move_to(&mut self, wanted: impl Fn(&TreeRow) -> bool) -> bool {
        let Some(at) = self.rows.iter().position(wanted) else {
            return false;
        };
        self.cursor = at;
        true
    }

    // ── quick search ───────────────────────────────────────────────

    /// The first folder whose name starts with `prefix`, from the top. The
    /// root is never a match: its "name" is a path, not something typed.
    pub fn find_first(&mut self, prefix: &str) -> bool {
        if prefix.is_empty() {
            return false;
        }
        self.move_to(|row| {
            row.branch != Branch::Root && starts_with_ignore_case(&row.name(), prefix)
        })
    }

    /// The next match after the cursor, wrapping — Far's `Ctrl+Enter`.
    pub fn find_next(&mut self, prefix: &str) -> bool {
        if prefix.is_empty() || self.rows.is_empty() {
            return false;
        }
        let len = self.rows.len();
        let Some(offset) = (1..=len).find(|offset| {
            let row = &self.rows[(self.cursor + offset) % len];
            row.branch != Branch::Root && starts_with_ignore_case(&row.name(), prefix)
        }) else {
            return false;
        };
        self.cursor = (self.cursor + offset) % len;
        true
    }
}

#[cfg(test)]
mod tests;
