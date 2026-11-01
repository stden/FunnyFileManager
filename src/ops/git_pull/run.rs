//! Purpose: Continue after repository failures and retain each result in the report.

use super::{
    discover,
    repositories::{self, Outcome},
};
use std::{fmt, io, path::Path};

#[derive(Default)]
pub(super) struct Summary {
    pub(super) updated: usize,
    pub(super) current: usize,
    pub(super) failed: usize,
    pub(super) skipped: usize,
    pub(super) scan_errors: usize,
    pub(super) cancelled: bool,
}

impl Summary {
    fn record(&mut self, outcome: Outcome) {
        match outcome {
            Outcome::Updated => self.updated += 1,
            Outcome::UpToDate => self.current += 1,
            Outcome::Failed => self.failed += 1,
            Outcome::Skipped => self.skipped += 1,
            Outcome::Cancelled => self.cancelled = true,
        }
    }
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = if self.cancelled {
            "cancelled"
        } else {
            "complete"
        };
        write!(
            f,
            "Git pull {state}: {} updated, {} up to date, {} failed, {} skipped, {} scan errors",
            self.updated, self.current, self.failed, self.skipped, self.scan_errors
        )
    }
}

pub(super) fn execute(
    root: &Path,
    mut emit: impl FnMut(&str) -> io::Result<()>,
    cancelled: &dyn Fn() -> bool,
) -> io::Result<Summary> {
    emit(&format!(
        "Search for Git repositories in {}",
        root.display()
    ))?;
    let scan = search(root, cancelled)?;
    let _progress = crate::progress::begin(crate::progress::Operation::GitPull, &scan.repositories);
    let mut summary = Summary {
        scan_errors: scan.errors,
        ..Summary::default()
    };
    if scan.repositories.is_empty() {
        emit("No Git repositories found")?;
    }
    for (index, repo) in scan.repositories.iter().enumerate() {
        let outcome = update(
            root,
            repo,
            index + 1,
            scan.repositories.len(),
            &mut emit,
            cancelled,
        )?;
        summary.record(outcome);
        crate::progress::source_done();
        if summary.cancelled {
            break;
        }
    }
    emit(&summary.to_string())?;
    Ok(summary)
}

fn update(
    root: &Path,
    repo: &Path,
    index: usize,
    total: usize,
    emit: &mut impl FnMut(&str) -> io::Result<()>,
    cancelled: &dyn Fn() -> bool,
) -> io::Result<Outcome> {
    crate::progress::entering(repo);
    let name = repo.strip_prefix(root).unwrap_or(repo);
    let name = if name.as_os_str().is_empty() {
        ".".to_owned()
    } else {
        name.display().to_string()
    };
    emit(&format!("[{index}/{total}] {name}: git pull --ff-only"))?;
    let result = repositories::pull(repo, cancelled);
    emit(&format!(
        "{name}: {}\n{}",
        result.outcome,
        result.output.trim()
    ))?;
    Ok(result.outcome)
}

fn search(root: &Path, cancelled: &dyn Fn() -> bool) -> io::Result<discover::Scan> {
    let _progress = crate::progress::begin(crate::progress::Operation::GitPull, &[]);
    discover::scan(root, cancelled)
}

#[cfg(test)]
mod tests;
