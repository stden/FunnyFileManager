//! Purpose: Pull each current branch without merges, rebases, stashes, or submodule changes.

use std::{
    fmt, io,
    path::Path,
    process::{Command, Output},
    time::Duration,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    Updated,
    UpToDate,
    Failed,
    Skipped,
    Cancelled,
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Updated => "Updated",
            Self::UpToDate => "Already up to date",
            Self::Failed => "Failed",
            Self::Skipped => "Skipped",
            Self::Cancelled => "Cancelled",
        })
    }
}

pub(super) struct Pull {
    pub(super) outcome: Outcome,
    pub(super) output: String,
}

const PULL_ARGS: &[&str] = &[
    "pull",
    "--ff-only",
    "--no-rebase",
    "--no-autostash",
    "--no-recurse-submodules",
];

pub(super) fn pull(repo: &Path, cancelled: &dyn Fn() -> bool) -> Pull {
    match pull_result(repo, cancelled) {
        Ok(result) => result,
        Err(error) => Pull {
            outcome: if error.kind() == io::ErrorKind::Interrupted {
                Outcome::Cancelled
            } else {
                Outcome::Failed
            },
            output: error.to_string(),
        },
    }
}

fn pull_result(repo: &Path, cancelled: &dyn Fn() -> bool) -> io::Result<Pull> {
    if cancelled() {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "Git pull cancelled",
        ));
    }
    if is_submodule(repo, cancelled)? {
        return Ok(Pull {
            outcome: Outcome::Skipped,
            output: "Submodule: keep the revision selected by its parent".to_owned(),
        });
    }
    let before = head(repo, cancelled)?;
    let output = git(repo, PULL_ARGS, cancelled)?;
    let outcome = pull_outcome(repo, before.as_deref(), &output, cancelled)?;
    Ok(Pull {
        outcome,
        output: format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    })
}

fn is_submodule(repo: &Path, cancelled: &dyn Fn() -> bool) -> io::Result<bool> {
    let output = git(
        repo,
        &["rev-parse", "--show-superproject-working-tree"],
        cancelled,
    )?;
    Ok(output.status.success() && !String::from_utf8_lossy(&output.stdout).trim().is_empty())
}

fn pull_outcome(
    repo: &Path,
    before: Option<&[u8]>,
    output: &Output,
    cancelled: &dyn Fn() -> bool,
) -> io::Result<Outcome> {
    if !output.status.success() {
        return Ok(Outcome::Failed);
    }
    Ok(if before == head(repo, cancelled)?.as_deref() {
        Outcome::UpToDate
    } else {
        Outcome::Updated
    })
}

fn head(repo: &Path, cancelled: &dyn Fn() -> bool) -> io::Result<Option<Vec<u8>>> {
    let output = git(repo, &["rev-parse", "--verify", "HEAD"], cancelled)?;
    Ok(output.status.success().then_some(output.stdout))
}

pub(in crate::ops) fn git(
    repo: &Path,
    args: &[&str],
    cancelled: &dyn Fn() -> bool,
) -> io::Result<Output> {
    let mut command = Command::new("git");
    command
        .args([
            "-c",
            "credential.interactive=false",
            "-c",
            "submodule.recurse=false",
        ])
        .args(args)
        .current_dir(repo)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "Never");
    for key in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_COMMON_DIR",
        "GIT_INDEX_FILE",
    ] {
        command.env_remove(key);
    }
    super::super::menu::process::capture_cancellable(
        &mut command,
        Duration::from_secs(120),
        &std::env::temp_dir(),
        cancelled,
    )
}

#[cfg(test)]
mod tests;
