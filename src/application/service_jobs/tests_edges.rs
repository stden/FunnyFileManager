//! Purpose: Coverage for worker slot polling, spawn failures, and live dispatch branches.

use std::path::Path;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use super::{Pending, Redraw, ServiceJobs, WorkerSlot, FORCE_SPAWN_FAILURE};
use crate::models::services::ai::AiDialog;
use crate::models::services::jira::JiraDialog;
use crate::models::services::mcp_remote::McpRemoteDialog;
use crate::models::{App, Overlay, Panel};
use crate::test_support::{EnvVarsGuard, CONFIG_HOME};
use crate::values::ai::{AiAccess, AiError, AiSettings, WorkspaceContext};
use crate::values::jira::{JiraAccess, JiraError, JiraReply, JiraRequest};
use crate::values::mcp_remote::{HttpMcpServer, McpAccess, McpRemoteError, McpReply};

struct SpawnFailGuard;

impl Drop for SpawnFailGuard {
    fn drop(&mut self) {
        FORCE_SPAWN_FAILURE.set(false);
    }
}

fn test_app(dir: &Path) -> App {
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
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("service job did not finish before deadline");
}

fn pending_slot<Id: Default, Rep>(fallback: Rep) -> (mpsc::Sender<Rep>, WorkerSlot<Id, Rep>) {
    let (tx, rx) = mpsc::channel();
    let slot = WorkerSlot {
        pending: Some(Pending {
            generation: Arc::new(()),
            id: Id::default(),
            fallback,
            receiver: rx,
        }),
    };
    (tx, slot)
}

fn ai_with_prompt(settings: AiSettings) -> AiDialog {
    let mut ai = AiDialog::new(settings);
    ai.prompt.set_text("hello");
    ai.queue_prompt(WorkspaceContext::default());
    ai
}

fn mcp_dialog(url: &str) -> McpRemoteDialog {
    let srv = HttpMcpServer {
        name: "svc".into(),
        url: url.into(),
        ..Default::default()
    };
    McpRemoteDialog::new(vec![srv])
}

#[test]
fn worker_slot_poll_and_pump_handle_empty_and_disconnected_channels() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = test_app(temp.path());
    let (tx, mut slot) = pending_slot::<u32, &'static str>("fb");
    assert_eq!(slot.poll(&mut app, |_, _, _, _| {}), Redraw::Unchanged);
    drop(tx);
    let mut received = None;
    assert_eq!(
        slot.poll(&mut app, |_, _, _, v| received = Some(v)),
        Redraw::Needed
    );
    assert_eq!(received, Some("fb"));
}

#[test]
fn service_jobs_pump_skips_slots_with_pending_workers() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = test_app(temp.path());
    let (jtx, jira) = pending_slot(JiraReply::Fetched(Err(JiraError::Worker)));
    let (mtx, mcp) = pending_slot(McpReply::Called(Err(McpRemoteError::Worker)));
    let (atx, ai) = pending_slot(Err(AiError::Worker));
    let mut jobs = ServiceJobs { jira, mcp, ai };
    assert_eq!(jobs.pump(&mut app), Redraw::Unchanged);
    drop((jtx, mtx, atx));
}

#[test]
fn spawn_failure_publishes_fallback_for_all_three_services() {
    let temp = tempfile::TempDir::new().expect("temp");
    let (mut app, mut jobs) = (test_app(temp.path()), ServiceJobs::default());
    let _guard = SpawnFailGuard;
    FORCE_SPAWN_FAILURE.set(true);

    let jira = JiraDialog::new_ready(
        "jql".into(),
        10,
        temp.path().to_path_buf(),
        "https://j.test".into(),
    );
    app.set_overlay(Overlay::Jira(Box::new(jira)));
    assert_eq!(jobs.pump(&mut app), Redraw::Needed);
    assert!(matches!(&app.overlay, Overlay::Jira(d) if !d.is_busy()));

    app.set_overlay(Overlay::McpServices(Box::new(mcp_dialog("https://m.test"))));
    assert_eq!(jobs.pump(&mut app), Redraw::Needed);
    assert!(matches!(&app.overlay, Overlay::McpServices(d) if !d.is_busy()));

    app.set_overlay(Overlay::Ai(Box::new(ai_with_prompt(AiSettings::default()))));
    assert_eq!(jobs.pump(&mut app), Redraw::Needed);
    assert!(matches!(&app.overlay, Overlay::Ai(d) if !d.is_busy()));
}

#[test]
fn network_mode_routes_jira_export_without_external_network() {
    let temp = tempfile::TempDir::new().expect("temp");
    let (mut app, mut jobs) = (test_app(temp.path()), ServiceJobs::default());
    app.live.jira = JiraAccess::Network;
    let mut jira = JiraDialog::new_setup(String::new(), "jql".into(), temp.path().to_path_buf());
    jira.queue(JiraRequest::ExportTickets {
        issues: Vec::new(),
        user: None,
        base_url: "https://jira.example.test".into(),
        output_dir: temp.path().to_path_buf(),
        force: false,
    });
    app.set_overlay(Overlay::Jira(Box::new(jira)));
    wait_until(
        &mut jobs,
        &mut app,
        |a| matches!(&a.overlay, Overlay::Jira(d) if !d.is_busy()),
    );
}

#[test]
fn network_mode_routes_mcp_without_external_network() {
    let temp = tempfile::TempDir::new().expect("temp");
    let (mut app, mut jobs) = (test_app(temp.path()), ServiceJobs::default());
    app.live.mcp_remote = McpAccess::Network;
    app.set_overlay(Overlay::McpServices(Box::new(mcp_dialog("http://bad"))));
    wait_until(
        &mut jobs,
        &mut app,
        |a| matches!(&a.overlay, Overlay::McpServices(d) if !d.is_busy()),
    );
}

#[test]
fn network_mode_routes_ai_without_external_network() {
    let temp = tempfile::TempDir::new().expect("temp");
    let _env = EnvVarsGuard::set(&[
        (CONFIG_HOME, temp.path()),
        ("ANTHROPIC_API_KEY", Path::new("")),
    ]);
    let (mut app, mut jobs) = (test_app(temp.path()), ServiceJobs::default());
    app.live.ai = AiAccess::Network;
    let settings = AiSettings {
        ca_cert: Some("/nonexistent/ca.pem".into()),
        ..Default::default()
    };
    app.set_overlay(Overlay::Ai(Box::new(ai_with_prompt(settings))));
    wait_until(
        &mut jobs,
        &mut app,
        |a| matches!(&a.overlay, Overlay::Ai(d) if !d.is_busy()),
    );
}
