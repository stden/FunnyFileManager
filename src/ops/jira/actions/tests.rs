//! Purpose: Unit tests for Jira overlay action guards and empty-state handling.

use super::{
    begin_edit_jql, queue_export_all, queue_export_selected, queue_snapshot, reload, submit_jql,
    view_selected,
};
use crate::lineedit::LineEditor;
use crate::models::services::jira::{JiraDialog, JiraMode};
use crate::models::{App, Overlay, Panel};

fn app_in(dir: &std::path::Path) -> App {
    let panel = Panel::new(dir.to_path_buf()).expect("panel");
    App::with_panels(panel.clone(), panel)
}

#[test]
fn jira_actions_no_op_when_jira_overlay_is_closed() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app_in(temp.path());
    view_selected(&mut app);
    queue_export_selected(&mut app, false);
    queue_export_all(&mut app, false);
    queue_snapshot(&mut app);
    begin_edit_jql(&mut app);
    submit_jql(&mut app);
    reload(&mut app);
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn jira_actions_report_status_when_no_issues_are_loaded() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app_in(temp.path());
    let dialog = JiraDialog::new_setup(
        String::new(),
        "project = DEMO".to_owned(),
        temp.path().to_path_buf(),
    );
    app.set_overlay(Overlay::Jira(Box::new(dialog)));

    view_selected(&mut app);
    queue_export_selected(&mut app, false);
    assert!(matches!(
        &app.overlay,
        Overlay::Jira(d) if d.status == "No Jira issue selected"
    ));
    queue_export_all(&mut app, false);
    assert!(matches!(
        &app.overlay,
        Overlay::Jira(d) if d.status == "No Jira issues loaded"
    ));
}

#[test]
fn submit_jql_ignores_browse_mode_and_blank_queries() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app_in(temp.path());
    let dialog = JiraDialog::new_setup(
        String::new(),
        "project = DEMO".to_owned(),
        temp.path().to_path_buf(),
    );
    app.set_overlay(Overlay::Jira(Box::new(dialog)));
    submit_jql(&mut app);
    if let Overlay::Jira(d) = &mut app.overlay {
        d.mode = JiraMode::EditJql(LineEditor::new("   "));
    }
    submit_jql(&mut app);
    assert!(matches!(
        &app.overlay,
        Overlay::Jira(d) if matches!(d.mode, JiraMode::EditJql(_)) && d.queued.is_none()
    ));
}
