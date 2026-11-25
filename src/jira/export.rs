//! Purpose: Export Jira stories into `{KEY}/` folders and atomic `jira_exports/{DATE}/` snapshots.

mod snapshot;

pub use snapshot::export_snapshot;
#[cfg(test)]
pub use snapshot::{export_snapshot_at, SnapshotStamp};

use std::path::Path;

use super::markdown::{jira_to_md, render_issue_md};
use crate::file_write::{create_dirs_owner_only, replace_owner_only, SyncPolicy};
use crate::values::jira::{ExportReport, JiraError, JiraIssue, JiraUser};

/// Reject anything other than a canonical `PROJECT-123` Jira key before building a path.
pub fn validate_issue_key(key: &str) -> Result<(), JiraError> {
    let Some((project, number)) = key.split_once('-') else {
        return Err(JiraError::Config(format!("Invalid Jira issue key: {key}")));
    };
    let valid_proj = project
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_uppercase)
        && project
            .as_bytes()
            .iter()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || *b == b'_');
    let valid_num = !number.is_empty() && number.as_bytes().iter().all(u8::is_ascii_digit);
    if !valid_proj || !valid_num {
        return Err(JiraError::Config(format!("Invalid Jira issue key: {key}")));
    }
    Ok(())
}

/// Write `{KEY}/desc.md`, `issue.md`, `comment.md`, and `comment.id` under `output_dir`.
pub fn export_tickets(
    output_dir: &Path,
    base_url: &str,
    user: Option<&JiraUser>,
    issues: &[JiraIssue],
    force: bool,
) -> Result<ExportReport, JiraError> {
    create_dirs_owner_only(output_dir).map_err(|err| JiraError::Io(err.to_string()))?;
    let (mut written, mut skipped) = (0usize, 0usize);
    for issue in issues {
        export_single_ticket(
            output_dir,
            base_url,
            user,
            issue,
            force,
            &mut written,
            &mut skipped,
        )?;
    }
    Ok(ExportReport {
        summary: format!(
            "Exported {} Jira ticket(s) to {} ({written} files written, {skipped} skipped)",
            issues.len(),
            output_dir.display()
        ),
        output_dir: output_dir.to_path_buf(),
        written,
        skipped,
    })
}

fn export_single_ticket(
    output_dir: &Path,
    base_url: &str,
    user: Option<&JiraUser>,
    issue: &JiraIssue,
    force: bool,
    written: &mut usize,
    skipped: &mut usize,
) -> Result<(), JiraError> {
    validate_issue_key(&issue.key)?;
    let ticket_dir = output_dir.join(&issue.key);
    create_dirs_owner_only(&ticket_dir).map_err(|err| JiraError::Io(err.to_string()))?;
    write_if_allowed(
        &ticket_dir.join("desc.md"),
        &jira_to_md(&issue.description),
        force,
        written,
        skipped,
    )?;
    write_if_allowed(
        &ticket_dir.join("issue.md"),
        &render_issue_md(base_url, issue),
        force,
        written,
        skipped,
    )?;
    export_own_comment(&ticket_dir, issue, user, force, written, skipped)
}

fn export_own_comment(
    ticket_dir: &Path,
    issue: &JiraIssue,
    user: Option<&JiraUser>,
    force: bool,
    written: &mut usize,
    skipped: &mut usize,
) -> Result<(), JiraError> {
    let Some(user) = user else {
        return Ok(());
    };
    let Some(comment) = issue
        .comments
        .iter()
        .rev()
        .find(|c| user.matches_author(&c.author_name, &c.author_key))
    else {
        return Ok(());
    };
    let (comment_path, id_path) = (ticket_dir.join("comment.md"), ticket_dir.join("comment.id"));
    if !force && comment_path.exists() && id_path.exists() {
        *skipped += 2;
        return Ok(());
    }
    write_file(&comment_path, &jira_to_md(&comment.body))?;
    write_file(&id_path, &comment.id)?;
    *written += 2;
    Ok(())
}

fn write_if_allowed(
    dest: &Path,
    content: &str,
    force: bool,
    written: &mut usize,
    skipped: &mut usize,
) -> Result<(), JiraError> {
    if dest.exists() && !force {
        *skipped += 1;
        return Ok(());
    }
    write_file(dest, content)?;
    *written += 1;
    Ok(())
}

pub(super) fn write_file(dest: &Path, content: &str) -> Result<(), JiraError> {
    replace_owner_only(dest, "jira", content, SyncPolicy::Skip)
        .map_err(|err| JiraError::Io(format!("{}: {err}", dest.display())))
}
