//! Bounded filesystem search and directory comparisons for menu commands.

use crate::{
    mask::Mask,
    menu::MenuCommand,
    models::{App, ChoiceTarget, MenuChoice},
};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub(super) fn find(app: &mut App, pattern: &str, folders: bool, containing: &str) {
    let root = app.get_active_panel_ref().path.clone();
    let mut search = Search::prepare(app, pattern, folders, containing);
    search.walk(&root);
    let title = if folders { "Find folder" } else { "Find file" };
    super::choices(app, title, search.rows);
    report(app, search.truncated, search.skipped);
}

struct Search {
    mask: Mask,
    folders: bool,
    needle: String,
    depth_limit: usize,
    budget: usize,
    truncated: bool,
    /// Folders whose listing could not be read, and so were not searched.
    skipped: usize,
    rows: Vec<MenuChoice>,
}

impl Search {
    fn prepare(app: &App, pattern: &str, folders: bool, containing: &str) -> Self {
        Self {
            mask: Mask::parse(pattern),
            folders,
            needle: containing.trim().to_lowercase(),
            depth_limit: app.settings.number(crate::menu::SettingsPage::Tree),
            budget: app.settings.number(crate::menu::SettingsPage::System),
            truncated: false,
            skipped: 0,
            rows: Vec::new(),
        }
    }

    fn walk(&mut self, root: &Path) {
        let mut pending = vec![(root.to_path_buf(), 0_usize)];
        while let Some((directory, depth)) = pending.pop() {
            if self.budget == 0 {
                self.truncated = true;
                break;
            }
            self.visit(root, &directory, depth, &mut pending);
            if self.truncated {
                break;
            }
        }
    }

    fn visit(
        &mut self,
        root: &Path,
        directory: &Path,
        depth: usize,
        pending: &mut Vec<(PathBuf, usize)>,
    ) {
        let Ok(entries) = read_directory(directory) else {
            self.skipped += 1;
            return;
        };
        for entry in entries.flatten() {
            if !self.take() {
                return;
            }
            self.consider(root, &entry, depth, pending);
        }
    }

    fn take(&mut self) -> bool {
        if self.budget == 0 {
            self.truncated = true;
            return false;
        }
        self.budget -= 1;
        true
    }

    fn consider(
        &mut self,
        root: &Path,
        entry: &fs::DirEntry,
        depth: usize,
        pending: &mut Vec<(PathBuf, usize)>,
    ) {
        let Ok(kind) = entry.file_type() else {
            return;
        };
        let path = entry.path();
        if kind.is_dir() && depth < self.depth_limit {
            pending.push((path.clone(), depth + 1));
        }
        let name = entry.file_name();
        let wanted = kind.is_dir() == self.folders && self.mask.matches(&name.to_string_lossy());
        if wanted && self.text_matches(&path) {
            add_result(root, path, self.folders, &mut self.rows);
        }
    }

    fn text_matches(&self, path: &Path) -> bool {
        self.folders || self.needle.is_empty() || file_contains(path, &self.needle)
    }
}

fn report(app: &mut App, truncated: bool, skipped: usize) {
    let unreadable = match skipped {
        1 => "skipped 1 folder that could not be read".to_owned(),
        count => format!("skipped {count} folders that could not be read"),
    };
    let message = match (truncated, skipped) {
        (false, 0) => return,
        (true, 0) => "Search stopped at the entry limit. The list is incomplete.".to_owned(),
        (false, _) => format!("Search {unreadable}."),
        (true, _) => format!("Search stopped at the entry limit and {unreadable}."),
    };
    app.set_status(crate::status::Status::CommandOutput(message));
}

fn read_directory(path: &Path) -> io::Result<fs::ReadDir> {
    #[cfg(test)]
    if denied(path) {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "denied"));
    }
    fs::read_dir(path)
}

#[cfg(test)]
thread_local! {
    static DENIED: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// Fail `read_dir` for a directory whose last component is `name`.
#[cfg(test)]
pub(super) struct DenyReads {
    previous: Option<String>,
}

#[cfg(test)]
impl DenyReads {
    pub(super) fn named(name: &str) -> Self {
        let previous = DENIED.with(|slot| slot.borrow_mut().replace(name.to_owned()));
        Self { previous }
    }
}

#[cfg(test)]
impl Drop for DenyReads {
    fn drop(&mut self) {
        DENIED.with(|slot| *slot.borrow_mut() = self.previous.take());
    }
}

#[cfg(test)]
fn denied(path: &Path) -> bool {
    DENIED.with(|slot| {
        slot.borrow()
            .as_ref()
            .is_some_and(|name| path.ends_with(name.as_str()))
    })
}

fn file_contains(path: &Path, needle: &str) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() || metadata.len() > crate::utils::MAX_FILE_SIZE.bytes() {
        return false;
    }
    let Ok(bytes) = fs::read(path) else {
        return false;
    };
    !bytes.contains(&0)
        && String::from_utf8_lossy(&bytes)
            .to_lowercase()
            .contains(needle)
}

fn add_result(root: &Path, path: PathBuf, folders: bool, rows: &mut Vec<MenuChoice>) {
    let label = path
        .strip_prefix(root)
        .unwrap_or(&path)
        .display()
        .to_string();
    let target = if folders {
        ChoiceTarget::Directory(path)
    } else {
        ChoiceTarget::File(path)
    };
    rows.push(MenuChoice { label, target });
}

pub(super) fn compare(app: &mut App) {
    let (left, right) = (&mut app.left_panel, &mut app.right_panel);
    left.marked.clear();
    right.marked.clear();
    mark_different(left, right);
    mark_different(right, left);
    app.set_status(crate::status::Status::CommandOutput(
        "Marked missing or different-sized files in both panels".to_owned(),
    ));
}

fn mark_different(panel: &mut crate::models::Panel, other: &crate::models::Panel) {
    for entry in &panel.entries {
        if !entry.kind.is_dir()
            && !other
                .entries
                .iter()
                .any(|candidate| candidate.name == entry.name && candidate.size == entry.size)
        {
            panel
                .marked
                .insert(crate::models::FileName::from(entry.name.as_str()));
        }
    }
}

pub(super) fn prompt(app: &mut App, command: MenuCommand) {
    let folders = matches!(command, MenuCommand::FindFolder);
    super::prompt(
        app,
        command,
        if folders { "Find folder" } else { "Find file" },
        "Name mask (recursive, maximum 100,000 entries)",
        "*",
    );
}
