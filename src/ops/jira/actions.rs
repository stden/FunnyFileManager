//! Purpose: Interactive key actions inside the open Jira browser overlay.

use crate::lineedit::LineEditor;
use crate::models::services::jira::JiraMode;
use crate::models::{App, Overlay};
use crate::values::jira::{JiraRequest, DEFAULT_SNAPSHOT_JQL};

/// Open the highlighted Jira issue in the scrollable detail reader.
pub fn view_selected(app: &mut App) {
    let Overlay::Jira(dialog) = &mut app.overlay else {
        return;
    };
    let Some(issue) = dialog.selected_issue().cloned() else {
        return;
    };
    let rendered = crate::jira::markdown::render_issue_md(&dialog.base_url, &issue);
    let title = format!("{} — {}", issue.key, issue.status);
    let lines = rendered.lines().map(str::to_owned).collect();
    dialog.mode = JiraMode::Detail {
        title,
        lines,
        scroll: 0,
    };
}

/// Queue `{KEY}/desc.md` + `issue.md` + `comment.md` export for marked/selected issues.
pub fn queue_export_selected(app: &mut App, force: bool) {
    let Overlay::Jira(dialog) = &mut app.overlay else {
        return;
    };
    let issues = dialog.selected_or_marked();
    if issues.is_empty() {
        "No Jira issue selected".clone_into(&mut dialog.status);
        return;
    }
    let user = dialog.user.clone();
    let base_url = dialog.base_url.clone();
    let output_dir = dialog.export_dir.clone();
    dialog.queue(JiraRequest::ExportTickets {
        issues,
        user,
        base_url,
        output_dir,
        force,
    });
}

/// Queue `{KEY}/` export for every currently loaded issue.
pub fn queue_export_all(app: &mut App, force: bool) {
    let Overlay::Jira(dialog) = &mut app.overlay else {
        return;
    };
    if dialog.issues.is_empty() {
        "No Jira issues loaded".clone_into(&mut dialog.status);
        return;
    }
    let issues = dialog.issues.clone();
    let user = dialog.user.clone();
    let base_url = dialog.base_url.clone();
    let output_dir = dialog.export_dir.clone();
    dialog.queue(JiraRequest::ExportTickets {
        issues,
        user,
        base_url,
        output_dir,
        force,
    });
}

/// Queue an atomic `jira_exports/{DATE}/` snapshot export (`issues.json`, `metadata.json`, `tickets.md`).
pub fn queue_snapshot(app: &mut App) {
    let Overlay::Jira(dialog) = &mut app.overlay else {
        return;
    };
    let page_size = dialog.page_size;
    let output_dir = dialog.export_dir.clone();
    let jql = DEFAULT_SNAPSHOT_JQL.to_owned();
    dialog.queue(JiraRequest::ExportSnapshot {
        jql,
        page_size,
        output_dir,
    });
}

/// Switch the Jira dialog into JQL editing mode.
pub fn begin_edit_jql(app: &mut App) {
    let Overlay::Jira(dialog) = &mut app.overlay else {
        return;
    };
    dialog.mode = JiraMode::EditJql(LineEditor::from(dialog.jql.as_str()));
}

/// Submit the edited JQL and reload issues from Jira.
pub fn submit_jql(app: &mut App) {
    let Overlay::Jira(dialog) = &mut app.overlay else {
        return;
    };
    let JiraMode::EditJql(editor) = &dialog.mode else {
        return;
    };
    let trimmed = editor.text().trim().to_owned();
    if trimmed.is_empty() {
        return;
    }
    dialog.jql.clone_from(&trimmed);
    dialog.mode = JiraMode::Browse;
    let page_size = dialog.page_size;
    dialog.queue(JiraRequest::Fetch {
        jql: trimmed,
        page_size,
    });
}

/// Reload the current JQL query from Jira.
pub fn reload(app: &mut App) {
    let Overlay::Jira(dialog) = &mut app.overlay else {
        return;
    };
    let jql = dialog.jql.clone();
    let page_size = dialog.page_size;
    dialog.queue(JiraRequest::Fetch { jql, page_size });
}

#[cfg(test)]
mod tests;
