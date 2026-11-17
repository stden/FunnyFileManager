//! Purpose: Find duplicate blocks (Commands → Find duplicate blocks): the dialog
//! that asks where to look, and the list of repeated blocks it opens.
//!
//! The folder walk is in `walk`; the repeats themselves come from the pure
//! `duplicates` module.

use std::path::{Path, PathBuf};

use crate::duplicates::{self, Group};
use crate::mask::Mask;
use crate::models::{App, ChoiceTarget, Duplicates, MenuChoice, Overlay};
use crate::status::Status;

use walk::{Budget, Limit};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_dialog;
mod walk;

/// The fewest non-blank lines a copy may be: a shorter block is a line or two.
const MIN_BLOCK_FLOOR: usize = 3;

/// Commands → Find duplicate blocks: open the dialog on the active folder.
pub fn open(app: &mut App) {
    let folder = app.get_active_panel_ref().path.display().to_string();
    app.set_overlay(Overlay::Duplicates(Box::new(Duplicates::new(&folder))));
}

/// Start: run the search the dialog asks for, or say why it cannot run.
pub fn accept(app: &mut App) {
    let Overlay::Duplicates(dialog) = &app.overlay else {
        return;
    };
    match Query::from_dialog(dialog) {
        Ok(query) => run(app, &query, Budget::standard()),
        Err(message) => app.set_status(Status::CommandOutput(message)),
    }
}

/// The dialog's answers, checked.
struct Query {
    folder: PathBuf,
    mask: Mask,
    min_block: usize,
    groups: usize,
}

impl Query {
    fn from_dialog(dialog: &Duplicates) -> Result<Self, String> {
        let folder = PathBuf::from(dialog.folder.text().trim());
        if !folder.is_dir() {
            return Err(format!("Not a folder: {}", folder.display()));
        }
        let min_block = number(dialog.min_block.text(), MIN_BLOCK_FLOOR).ok_or_else(|| {
            format!("Minimum block must be a number of lines, {MIN_BLOCK_FLOOR} or more.")
        })?;
        let groups = number(dialog.groups.text(), 1)
            .ok_or_else(|| "Groups must be a number, 1 or more.".to_owned())?;
        let pattern = dialog.mask.text().trim();
        let mask = Mask::parse(if pattern.is_empty() { "*" } else { pattern });
        Ok(Self {
            folder,
            mask,
            min_block,
            groups,
        })
    }
}

fn number(text: &str, least: usize) -> Option<usize> {
    text.trim()
        .parse::<usize>()
        .ok()
        .filter(|value| *value >= least)
}

/// Walk the query's folder under `budget`, then open its repeats as a list.
/// A walk or a comparison that ran out of budget says which limit did it.
fn run(app: &mut App, query: &Query, budget: Budget) {
    let _run = crate::progress::begin(
        crate::progress::Operation::Find,
        std::slice::from_ref(&query.folder),
    );
    let found = walk::walk(&query.folder, &query.mask, budget);
    let texts: Vec<&str> = found.files.iter().map(|(_, text)| text.as_str()).collect();
    let search = duplicates::find(&texts, query.min_block, query.groups, budget.deadline);
    let rows = rows(&query.folder, &found.files, &search.groups);
    crate::ops::menu::choices(app, "Find duplicate blocks", rows);
    let stopped = found
        .stopped
        .or_else(|| (!search.complete).then_some(Limit::Time));
    if let Some(limit) = stopped {
        app.set_status(Status::CommandOutput(stopped_message(limit)));
    }
}

/// One row per occurrence, in rank order: the rank, the group's size and
/// where the occurrence starts. Enter opens the file at that line.
fn rows(root: &Path, files: &[(PathBuf, String)], groups: &[Group]) -> Vec<MenuChoice> {
    groups
        .iter()
        .enumerate()
        .flat_map(|(rank, group)| {
            group.occurrences.iter().filter_map(move |occurrence| {
                let path = files.get(occurrence.source)?.0.as_path();
                Some(MenuChoice {
                    label: format!(
                        "{:>3}. {}x {} lines  {}:{}-{}",
                        rank + 1,
                        group.occurrences.len(),
                        group.block_lines,
                        path.strip_prefix(root).unwrap_or(path).display(),
                        occurrence.first_line,
                        occurrence.last_line,
                    ),
                    target: ChoiceTarget::FileLine {
                        path: path.to_path_buf(),
                        line: occurrence.first_line,
                    },
                })
            })
        })
        .collect()
}

fn stopped_message(limit: Limit) -> String {
    let which = match limit {
        Limit::Files => "file limit",
        Limit::Time => "time limit",
    };
    format!("Duplicate search stopped at the {which}. The list is incomplete.")
}
