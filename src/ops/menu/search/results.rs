//! Purpose: What a search hands back: its rows, and the status line that
//! says when the list is incomplete and why.

use std::path::{Path, PathBuf};

use crate::models::{App, ChoiceTarget, MenuChoice};

/// Which limit ended a search before it had looked everywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Limit {
    Entries,
    Time,
}

pub(super) fn report(app: &mut App, stopped: Option<Limit>, skipped: usize) {
    let unreadable = match skipped {
        1 => "skipped 1 folder that could not be read".to_owned(),
        count => format!("skipped {count} folders that could not be read"),
    };
    let limit = stopped.map(|limit| match limit {
        Limit::Entries => "entry limit",
        Limit::Time => "time limit",
    });
    let message = match (limit, skipped) {
        (None, 0) => return,
        (Some(limit), 0) => format!("Search stopped at the {limit}. The list is incomplete."),
        (None, _) => format!("Search {unreadable}."),
        (Some(limit), _) => format!("Search stopped at the {limit} and {unreadable}."),
    };
    app.set_status(crate::status::Status::CommandOutput(message));
}

pub(super) fn add_result(root: &Path, path: PathBuf, folders: bool, rows: &mut Vec<MenuChoice>) {
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
