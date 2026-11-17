//! Worker startup and channel failures reach the live dialog without network leakage.

use std::{path::Path, sync::mpsc, time::Duration};

use super::{app_with, read_ok, state, waiting, Config, ThingsJobs, ThingsState};
use crate::application::Redraw;
use crate::models::{things::ThingsAccess, Overlay};
use crate::things::ThingsError;

#[test]
fn closing_the_dialog_prevents_the_queued_read_from_starting() {
    let mut app = app_with(ThingsState::Queued(super::settings()));
    app.close_overlay();
    let mut jobs = ThingsJobs::default();
    assert!(matches!(
        jobs.pump_with(&mut app, |_config| panic!("closed dialog must not fetch")),
        Redraw::Unchanged
    ));
    assert!(jobs.pending.is_none());
}

#[test]
fn denied_worker_creation_fails_the_dialog_without_leaving_a_pending_read() {
    let mut app = app_with(ThingsState::Queued(super::settings()));
    let mut jobs = ThingsJobs::default();
    let redraw = jobs.pump_using(&mut app, read_ok, |_sender, _config, _work| {
        Err(std::io::Error::other("thread creation denied"))
    });
    assert!(matches!(redraw, Redraw::Needed));
    assert!(
        matches!(state(&app), ThingsState::Failed(text) if text == ThingsError::Worker.message())
    );
    assert!(jobs.pending.is_none());
}

#[test]
fn a_disconnected_worker_publishes_a_failure_and_clears_the_pending_read() {
    let mut app = app_with(ThingsState::Loading);
    let Overlay::Things(dialog) = &app.overlay else {
        panic!("catalog dialog");
    };
    let (sender, receiver) = mpsc::channel();
    drop(sender);
    let mut jobs = ThingsJobs {
        pending: Some(super::super::Pending {
            generation: std::sync::Arc::clone(&dialog.generation),
            receiver,
        }),
    };
    assert!(matches!(jobs.collect(&mut app), Redraw::Needed));
    assert!(
        matches!(state(&app), ThingsState::Failed(text) if text == ThingsError::Worker.message())
    );
    assert!(jobs.pending.is_none());
}

#[test]
fn live_network_reads_report_local_connection_failure_through_the_worker() {
    let _proxy = crate::test_support::EnvVarGuard::set("NO_PROXY", Path::new("*"));
    let config = Config::from_values(
        Some("https://127.0.0.1:0".to_owned()),
        Some("fixture-secret".to_owned()),
    )
    .expect("loopback catalog");
    let mut app = app_with(ThingsState::Queued(config));
    app.live.things = ThingsAccess::Network;
    let mut jobs = ThingsJobs::default();
    for _ in 0..5000 {
        jobs.pump(&mut app);
        if !waiting(&app) {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(
        matches!(state(&app), ThingsState::Failed(text) if text == ThingsError::Request.message())
    );
}
