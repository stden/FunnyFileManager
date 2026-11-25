//! Purpose: Atomic date-stamped Jira snapshot export (`metadata.json`, `issues.json`, `tickets.md`).

use std::fs;
use std::path::Path;

use jiff::ZonedDateTime;
use serde_json::json;

use super::write_file;
use crate::file_write::create_dirs_owner_only;
use crate::jira::markdown::render_tickets_index;
use crate::values::jira::{ExportReport, JiraError, JiraIssue, JiraUser};

/// Explicit-timestamp snapshot helper used by tests and `export_snapshot`.
#[cfg(test)]
pub struct SnapshotStamp<'a> {
    pub date_stamp: &'a str,
    pub time_stamp: &'a str,
    pub exported_at: &'a str,
}

#[cfg(not(test))]
struct SnapshotStamp<'a> {
    date_stamp: &'a str,
    time_stamp: &'a str,
    exported_at: &'a str,
}

/// Publish an atomic snapshot under `{output_dir}/jira_exports/{YYYY-MM-DD[-HHMMSS]}/`.
pub fn export_snapshot(
    output_dir: &Path,
    base_url: &str,
    user: &JiraUser,
    issues: &[JiraIssue],
    reported_total: usize,
    jql: &str,
) -> Result<ExportReport, JiraError> {
    let now = ZonedDateTime::now();
    let (date_stamp, time_stamp, exported_at) = (
        now.strftime("%Y-%m-%d").to_string(),
        now.strftime("%H%M%S").to_string(),
        now.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string(),
    );
    let stamp = SnapshotStamp {
        date_stamp: &date_stamp,
        time_stamp: &time_stamp,
        exported_at: &exported_at,
    };
    let meta = SnapshotMeta {
        base_url,
        user,
        issues,
        reported_total,
        jql,
    };
    export_snapshot_with(output_dir, &meta, &stamp)
}

/// Inputs describing a snapshot to write atomically.
pub struct SnapshotMeta<'a> {
    pub base_url: &'a str,
    pub user: &'a JiraUser,
    pub issues: &'a [JiraIssue],
    pub reported_total: usize,
    pub jql: &'a str,
}

#[cfg(test)]
pub fn export_snapshot_at(
    output_dir: &Path,
    base_url: &str,
    user: &JiraUser,
    issues: &[JiraIssue],
    reported_total: usize,
    jql: &str,
    stamp: &SnapshotStamp<'_>,
) -> Result<ExportReport, JiraError> {
    let meta = SnapshotMeta {
        base_url,
        user,
        issues,
        reported_total,
        jql,
    };
    export_snapshot_with(output_dir, &meta, stamp)
}

fn export_snapshot_with(
    output_dir: &Path,
    meta: &SnapshotMeta<'_>,
    stamp: &SnapshotStamp<'_>,
) -> Result<ExportReport, JiraError> {
    let root = output_dir.join("jira_exports");
    create_dirs_owner_only(&root).map_err(|err| JiraError::Io(err.to_string()))?;
    let folder = if root.join(stamp.date_stamp).exists() {
        format!("{}-{}", stamp.date_stamp, stamp.time_stamp)
    } else {
        stamp.date_stamp.to_owned()
    };
    let (dest, staging) = (root.join(&folder), root.join(format!(".{folder}.tmp")));
    if dest.exists() || staging.exists() {
        return Err(JiraError::Io(format!(
            "Refusing to overwrite existing snapshot directory {}",
            dest.display()
        )));
    }
    create_dirs_owner_only(&staging).map_err(|err| JiraError::Io(err.to_string()))?;
    write_snapshot_files(&staging, meta, stamp.exported_at)?;
    fs::rename(&staging, &dest).map_err(|err| JiraError::Io(err.to_string()))?;
    Ok(ExportReport {
        summary: format!(
            "Saved Jira snapshot ({} issues) to {}",
            meta.reported_total,
            dest.display()
        ),
        output_dir: dest,
        written: 3,
        skipped: 0,
    })
}

fn write_snapshot_files(
    staging: &Path,
    meta: &SnapshotMeta<'_>,
    exported_at: &str,
) -> Result<(), JiraError> {
    let metadata = json!({
        "exported_at": exported_at,
        "jira_url": meta.base_url,
        "authenticated_user": {
            "name": meta.user.name,
            "key": meta.user.key,
            "display_name": meta.user.display_name,
        },
        "jql": meta.jql,
        "jira_reported_total": meta.reported_total,
        "retrieved_count": meta.issues.len(),
        "contents": {
            "issues.json": "Full Jira search records with all returned fields",
            "tickets.md": "Human-readable ticket index",
        },
    });
    let mut meta_text =
        serde_json::to_string_pretty(&metadata).map_err(|err| JiraError::Io(err.to_string()))?;
    meta_text.push('\n');
    let raw: Vec<&serde_json::Value> = meta.issues.iter().map(|i| &i.raw).collect();
    let mut issues_text =
        serde_json::to_string_pretty(&raw).map_err(|err| JiraError::Io(err.to_string()))?;
    issues_text.push('\n');
    let index = render_tickets_index(meta.base_url, meta.user, meta.issues, exported_at, meta.jql);
    write_file(&staging.join("metadata.json"), &meta_text)?;
    write_file(&staging.join("issues.json"), &issues_text)?;
    write_file(&staging.join("tickets.md"), &index)
}
