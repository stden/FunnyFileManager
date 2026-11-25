//! Purpose: Unit tests for `ServiceJobs` offline safety and worker reply delivery.

use std::time::{Duration, Instant};

use super::*;
use crate::models::services::ai::AiDialog;
use crate::models::services::jira::JiraDialog;
use crate::models::services::mcp_remote::McpRemoteDialog;
use crate::models::{Overlay, Panel};
use crate::values::ai::{AiSettings, WorkspaceContext};
use crate::values::mcp_remote::HttpMcpServer;

fn test_app(dir: &std::path::Path) -> App {
    let panel = Panel::new(dir.to_path_buf()).expect("panel");
    App::with_panels(panel.clone(), panel)
}

fn wait_until(jobs: &mut ServiceJobs, app: &mut App, done: impl Fn(&App) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        let _ = jobs.pump(app);
        if done(app) {
            return;
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("service job did not finish before deadline");
}

#[test]
fn offline_jira_job_fails_without_network() {
    let temp = tempfile::TempDir::new().expect("temp");
    let (mut app, mut jobs) = (test_app(temp.path()), ServiceJobs::default());
    let jira = JiraDialog::new_ready(
        "jql".into(),
        10,
        temp.path().to_path_buf(),
        "https://jira.example.test".into(),
    );
    app.set_overlay(Overlay::Jira(Box::new(jira)));
    wait_until(
        &mut jobs,
        &mut app,
        |a| matches!(&a.overlay, Overlay::Jira(d) if !d.is_busy()),
    );
}

#[test]
fn offline_mcp_and_ai_jobs_fail_without_network() {
    let temp = tempfile::TempDir::new().expect("temp");
    let (mut app, mut jobs) = (test_app(temp.path()), ServiceJobs::default());
    let srv = HttpMcpServer {
        name: "svc".into(),
        url: "https://mcp.example.test".into(),
        ..HttpMcpServer::default()
    };
    app.set_overlay(Overlay::McpServices(Box::new(McpRemoteDialog::new(vec![
        srv,
    ]))));
    wait_until(
        &mut jobs,
        &mut app,
        |a| matches!(&a.overlay, Overlay::McpServices(d) if !d.is_busy()),
    );
    let mut ai = AiDialog::new(AiSettings::default());
    ai.prompt.set_text("hello");
    ai.queue_prompt(WorkspaceContext::default());
    app.set_overlay(Overlay::Ai(Box::new(ai)));
    wait_until(
        &mut jobs,
        &mut app,
        |a| matches!(&a.overlay, Overlay::Ai(d) if !d.is_busy()),
    );
}
