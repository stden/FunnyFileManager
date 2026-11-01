//! Purpose: The folder tree behind the Tree panel (`Ctrl+T`).
//!
//! The tree is expanded lazily, one level at a time, through a lister the
//! caller injects — nothing here reads the disk, so the shape of the tree can
//! be tested against a made-up filesystem and drawn without touching a real
//! one.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::quicksearch::QuickSearch;

mod cursor;
mod shape;

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
        self.name_cow().into_owned()
    }

    /// Borrowed folder name when valid UTF-8, avoiding per-row heap allocation
    /// during quick-search scans over thousands of expanded tree rows.
    pub(super) fn name_cow(&self) -> Cow<'_, str> {
        self.path.file_name().map_or_else(
            || Cow::Owned(self.path.display().to_string()),
            |name| name.to_string_lossy(),
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
}

#[cfg(test)]
mod tests;
