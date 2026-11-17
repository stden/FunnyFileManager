//! Purpose: Cloudflare requests run off the event loop. An answer reaches only the dialog that asked
//! for it, while its request is still the latest one, and a worker that dies is reported.

use super::*;
use crate::models::cloudflare_dns::{Busy, Zone, ZoneId};
use crate::models::Panel;
use std::sync::mpsc as channel;
use std::time::{Duration, Instant};

mod guard;
mod in_flight;

fn app(temp: &tempfile::TempDir) -> App {
    let panel = Panel::from_entries(temp.path().to_path_buf(), Vec::new());
    App::with_panels(panel.clone(), panel)
}

fn zones_reply() -> Reply {
    Reply::Zones(Ok(vec![Zone {
        id: ZoneId::new("0000000000000000000000000000000a"),
        name: "example.com".to_owned(),
    }]))
}

/// A fetch that waits for the test to release it, then answers with the zone list.
fn held(gate: channel::Receiver<()>) -> impl FnOnce(Request) -> Reply + Send + 'static {
    move |_| {
        gate.recv_timeout(Duration::from_secs(5))
            .expect("the test releases the request");
        zones_reply()
    }
}

/// Waits until the job has no request in flight, applying what came back.
fn finish(jobs: &mut CloudflareJobs, app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while jobs.pending.is_some() {
        jobs.poll(app);
        assert!(Instant::now() < deadline, "the request did not finish");
        std::thread::yield_now();
    }
}

fn panel(app: &App) -> &CloudflareDnsPanel {
    let Overlay::CloudflareDns(panel) = &app.overlay else {
        panic!("the dialog");
    };
    panel
}

#[test]
fn an_offline_session_answers_without_touching_the_network() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    crate::ops::cloudflare_dns::open(&mut app);
    let mut jobs = CloudflareJobs::default();
    jobs.pump(&mut app);
    finish(&mut jobs, &mut app);
    assert_eq!(panel(&app).status, CloudflareError::Disabled.message());
    assert_eq!(panel(&app).busy, None);
}

#[test]
fn the_answer_is_applied_to_the_dialog_that_asked() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    crate::ops::cloudflare_dns::open(&mut app);
    let mut jobs = CloudflareJobs::default();
    jobs.pump_with(&mut app, |_| zones_reply());
    finish(&mut jobs, &mut app);
    assert_eq!(panel(&app).zones.len(), 1);
}

#[test]
fn the_dialog_is_loading_until_the_answer_arrives() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    crate::ops::cloudflare_dns::open(&mut app);
    let (release, gate) = channel::channel();
    let mut jobs = CloudflareJobs::default();
    jobs.pump_with(&mut app, held(gate));
    assert_eq!(panel(&app).busy, Some(Busy::Zones));
    release.send(()).expect("released");
    finish(&mut jobs, &mut app);
    assert_eq!(panel(&app).zones.len(), 1);
}

#[test]
fn a_closed_dialog_takes_no_answer() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    crate::ops::cloudflare_dns::open(&mut app);
    let (release, gate) = channel::channel();
    let mut jobs = CloudflareJobs::default();
    jobs.pump_with(&mut app, held(gate));
    app.close_overlay();
    release.send(()).expect("released");
    finish(&mut jobs, &mut app);
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn a_reopened_dialog_takes_no_answer_for_the_request_of_the_one_before() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    crate::ops::cloudflare_dns::open(&mut app);
    let (release, gate) = channel::channel();
    let mut jobs = CloudflareJobs::default();
    jobs.pump_with(&mut app, held(gate));
    app.close_overlay();
    crate::ops::cloudflare_dns::open(&mut app);
    release.send(()).expect("released");
    finish(&mut jobs, &mut app);
    assert!(panel(&app).zones.is_empty(), "the older answer is dropped");
    assert_eq!(
        panel(&app).busy,
        Some(Busy::Zones),
        "the new dialog still waits"
    );
}

#[test]
fn a_superseded_request_is_dropped_when_a_newer_one_is_queued() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    crate::ops::cloudflare_dns::open(&mut app);
    let (release, gate) = channel::channel();
    let mut jobs = CloudflareJobs::default();
    jobs.pump_with(&mut app, held(gate));
    if let Overlay::CloudflareDns(panel) = &mut app.overlay {
        panel.settle();
        panel.enqueue(Request::Host);
    }
    release.send(()).expect("released");
    finish(&mut jobs, &mut app);
    assert!(
        panel(&app).zones.is_empty(),
        "the superseded answer is dropped"
    );
    assert_eq!(panel(&app).busy, Some(Busy::Host));
}

#[test]
fn a_worker_that_dies_is_reported_as_a_worker_failure() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    crate::ops::cloudflare_dns::open(&mut app);
    let mut jobs = CloudflareJobs::default();
    jobs.pump_with(&mut app, |_| panic!("the worker failed"));
    finish(&mut jobs, &mut app);
    assert_eq!(panel(&app).status, CloudflareError::Worker.message());
    assert_eq!(panel(&app).busy, None);
}

#[test]
fn pumping_without_the_dialog_starts_nothing() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    let mut jobs = CloudflareJobs::default();
    jobs.pump_with(&mut app, |_| panic!("no request is queued"));
    assert!(jobs.pending.is_none());
}

#[test]
fn a_worker_the_system_refuses_is_reported_on_the_dialog() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    let mut dialog = CloudflareDnsPanel::new();
    dialog.enqueue(Request::Zones);
    app.set_overlay(Overlay::CloudflareDns(Box::new(dialog)));
    REFUSE_SPAWN.with(|cell| cell.set(true));
    let mut jobs = CloudflareJobs::default();
    jobs.pump_with(&mut app, |_| zones_reply());
    assert!(jobs.pending.is_none());
    assert_eq!(panel(&app).status, CloudflareError::Worker.message());
    assert!(panel(&app).busy.is_none());
}
