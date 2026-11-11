//! Purpose: Flattens the expanded folder tree into display rows and O(1) parent/sibling links.

use std::path::{Path, PathBuf};

use super::{Branch, Expansion, FolderTree, Guide, RowLinks, TreeRow};

#[derive(Default)]
struct LayoutBuf {
    guides: Vec<Guide>,
    rows: Vec<TreeRow>,
    links: Vec<RowLinks>,
}

impl FolderTree {
    /// Lay the rows out again from the root. The cursor is clamped, not
    /// reset: a refresh that removed rows below it should not send it home.
    pub(super) fn rebuild(&mut self) {
        let mut buf = LayoutBuf::default();
        if !self.root.as_os_str().is_empty() {
            let root = self.root.clone();
            self.push_rows(&root, Branch::Root, None, &mut buf);
        }
        self.rows = buf.rows;
        self.links = buf.links;
        self.search_cache.clear();
        self.cursor = self.cursor.min(self.rows.len().saturating_sub(1));
    }

    fn expansion_of(&self, path: &Path) -> Expansion {
        match self.children.get(path) {
            Some(kids) if kids.is_empty() => Expansion::Leaf,
            Some(_) if self.expanded.contains(path) => Expansion::Expanded,
            _ => Expansion::Collapsed,
        }
    }

    fn push_rows(
        &self,
        path: &Path,
        branch: Branch,
        parent: Option<usize>,
        buf: &mut LayoutBuf,
    ) -> usize {
        let me = buf.rows.len();
        let expansion = self.expansion_of(path);
        buf.rows.push(TreeRow {
            path: path.to_path_buf(),
            guides: buf.guides.clone(),
            branch,
            expansion,
        });
        buf.links.push(RowLinks {
            parent,
            prev_sibling: None,
            next_sibling: None,
        });
        if let Some(kids) = (expansion == Expansion::Expanded)
            .then(|| self.children.get(path))
            .flatten()
        {
            self.push_children(me, branch, kids, buf);
        }
        me
    }

    fn push_children(&self, parent: usize, branch: Branch, kids: &[PathBuf], buf: &mut LayoutBuf) {
        let pushed = match branch {
            Branch::Root => false,
            Branch::Middle => {
                buf.guides.push(Guide::Line);
                true
            }
            Branch::Last => {
                buf.guides.push(Guide::Gap);
                true
            }
        };
        let last = kids.len().saturating_sub(1);
        let mut prev = None;
        for (index, kid) in kids.iter().enumerate() {
            let child_branch = if index == last {
                Branch::Last
            } else {
                Branch::Middle
            };
            let child_row = self.push_rows(kid, child_branch, Some(parent), buf);
            link_siblings(&mut buf.links, prev, child_row);
            prev = Some(child_row);
        }
        if pushed {
            buf.guides.pop();
        }
    }
}

fn link_siblings(links: &mut [RowLinks], prev: Option<usize>, curr: usize) {
    let Some(prev_row) = prev else {
        return;
    };
    if let Some(link) = links.get_mut(prev_row) {
        link.next_sibling = Some(curr);
    }
    if let Some(link) = links.get_mut(curr) {
        link.prev_sibling = Some(prev_row);
    }
}
