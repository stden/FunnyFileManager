//! Bounded filesystem search for menu commands.

use crate::command::language::SettingsPage;
use crate::mask::Mask;
use crate::models::{App, MenuChoice, MenuPrompt};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::{fs, io};

#[cfg(test)]
mod deny_reads;
mod results;
#[cfg(test)]
mod tests;
mod text;
use results::{add_result, report, Limit};
use text::{fold, Scan, Scanner};
/// How long one search may take. The walk blocks the event loop, and a
/// file's text is now read to its end however large it is, so this is what
/// keeps a search over a disk of big logs from freezing the screen.
const TIME_LIMIT: Duration = Duration::from_secs(10);

#[cfg(test)]
use deny_reads::denied;
#[cfg(test)]
pub(super) use deny_reads::DenyReads;

pub(super) fn find(app: &mut App, pattern: &str, folders: bool, containing: &str) {
    find_within(app, pattern, folders, containing, TIME_LIMIT);
}

/// [`find`], given `time` rather than [`TIME_LIMIT`].
pub(super) fn find_within(
    app: &mut App,
    pattern: &str,
    folders: bool,
    containing: &str,
    time: Duration,
) {
    let root = app.get_active_panel_ref().path.clone();
    let mut search = Search::prepare(app, pattern, folders, containing, Instant::now() + time);
    let _run = crate::progress::begin(
        crate::progress::Operation::Find,
        std::slice::from_ref(&root),
    );
    search.walk(&root);
    let title = if folders { "Find folder" } else { "Find file" };
    super::choices(app, title, search.rows);
    report(app, search.stopped, search.skipped);
}

struct Search {
    mask: Mask,
    folders: bool,
    scanner: Scanner,
    depth_limit: usize,
    budget: usize,
    deadline: Instant,
    stopped: Option<Limit>,
    /// Folders whose listing could not be read, and so were not searched.
    skipped: usize,
    rows: Vec<MenuChoice>,
}

impl Search {
    fn prepare(
        app: &App,
        pattern: &str,
        folders: bool,
        containing: &str,
        deadline: Instant,
    ) -> Self {
        Self {
            mask: Mask::parse(pattern),
            folders,
            scanner: Scanner::new(fold(containing.trim())),
            depth_limit: app.settings.number(SettingsPage::Tree),
            budget: app.settings.number(SettingsPage::System),
            deadline,
            stopped: None,
            skipped: 0,
            rows: Vec::new(),
        }
    }

    fn walk(&mut self, root: &Path) {
        let mut pending = vec![(root.to_path_buf(), 0_usize)];
        while let Some((directory, depth)) = pending.pop() {
            if self.limit_reached() {
                break;
            }
            self.visit(root, &directory, depth, &mut pending);
            if self.stopped.is_some() {
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
        crate::progress::entering(directory);
        let Ok(entries) = read_directory(directory) else {
            self.skipped += 1;
            return;
        };
        for entry in entries.flatten() {
            if !self.consume_budget() {
                return;
            }
            self.consider(root, &entry, entry.file_type().ok(), depth, pending);
        }
    }

    /// Spend one entry of the budget, unless a limit has ended the search.
    fn consume_budget(&mut self) -> bool {
        if self.limit_reached() {
            return false;
        }
        self.budget -= 1;
        crate::progress::examined();
        true
    }

    /// Whether the entry budget or the time is used up; notes which.
    fn limit_reached(&mut self) -> bool {
        if self.budget == 0 {
            self.stopped = Some(Limit::Entries);
        } else if Instant::now() >= self.deadline {
            self.stopped = Some(Limit::Time);
        }
        self.stopped.is_some()
    }

    fn consider(
        &mut self,
        root: &Path,
        entry: &fs::DirEntry,
        kind: Option<fs::FileType>,
        depth: usize,
        pending: &mut Vec<(PathBuf, usize)>,
    ) {
        let Some(kind) = kind else { return };
        if kind.is_dir() && depth < self.depth_limit {
            pending.push((entry.path(), depth + 1));
        }
        let matches_name =
            self.mask.matches_all() || self.mask.matches(&entry.file_name().to_string_lossy());
        if kind.is_dir() == self.folders && matches_name {
            let path = entry.path();
            if self.text_matches(&path, Some(kind)) {
                crate::progress::matched();
                add_result(root, path, self.folders, &mut self.rows);
            }
        }
    }

    fn text_matches(&mut self, path: &Path, kind: Option<fs::FileType>) -> bool {
        if self.folders || self.scanner.is_empty() {
            return true;
        }
        crate::progress::entering(path);
        match self.scanner.file_contains(path, kind, self.deadline) {
            Scan::Found => true,
            Scan::Absent => false,
            Scan::OutOfTime => {
                self.stopped = Some(Limit::Time);
                false
            }
        }
    }
}

fn read_directory(path: &Path) -> io::Result<fs::ReadDir> {
    #[cfg(test)]
    if denied(path) {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "denied"));
    }
    fs::read_dir(path)
}

/// Alt+F10: the folder mask prompt. Find file (Alt+F7) has a dialog of
/// its own, with a containing-text field.
pub(super) fn prompt(app: &mut App) {
    super::prompt(
        app,
        MenuPrompt::FindFolder,
        "Find folder",
        "Name mask (recursive, maximum 100,000 entries)",
        "*",
    );
}
