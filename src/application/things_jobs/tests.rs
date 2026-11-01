//! Unit tests: a queued read is answered on the worker and shown; a refused
//! read fails the dialog; offline reads are refused without a request; and a
//! reply for a dialog that was replaced is dropped.

use std::time::Duration;

use super::{Answer, ThingsJobs};
use crate::models::things::{ThingsDialog, ThingsState};
use crate::models::{App, Overlay, Panel};
use crate::things::{Config, Listing, Machine, ThingsError};

fn settings() -> Config {
    Config::from_values(
        Some("https://things.example.com".to_owned()),
        Some("test-token-123".to_owned()),
    )
    .expect("settings")
}

fn app_with(state: ThingsState) -> App {
    let panel = Panel::from_entries(std::env::temp_dir(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    app.set_overlay(Overlay::Things(Box::new(ThingsDialog::new(state))));
    app
}

fn read_ok(_config: Config) -> Answer {
    let machine = Machine {
        id: "1".to_owned(),
        name: "web-01".to_owned(),
        hostname: "web-01.example.com".to_owned(),
        ..Machine::default()
    };
    Ok(Listing::new(vec![machine], None))
}

fn read_refused(_config: Config) -> Answer {
    Err(ThingsError::Request)
}

fn state(app: &App) -> &ThingsState {
    let Overlay::Things(dialog) = &app.overlay else {
        panic!("the dialog is open");
    };
    &dialog.state
}

fn waiting(app: &App) -> bool {
    matches!(state(app), ThingsState::Queued(_) | ThingsState::Loading)
}

/// Pump until the dialog is no longer waiting for its read, however long that
/// takes on a busy machine, within a bound.
fn settle(jobs: &mut ThingsJobs, app: &mut App, fetch: fn(Config) -> Answer) {
    for _ in 0..5000 {
        jobs.pump_with(app, fetch);
        if !waiting(app) {
            return;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    panic!("the read never answered");
}

#[test]
fn a_queued_read_is_answered_on_the_worker_and_shown() {
    let mut app = app_with(ThingsState::Queued(settings()));
    settle(&mut ThingsJobs::default(), &mut app, read_ok);
    assert!(matches!(state(&app), ThingsState::Ready(_)));
}

#[test]
fn a_refused_read_fails_the_dialog_with_its_message() {
    let mut app = app_with(ThingsState::Queued(settings()));
    settle(&mut ThingsJobs::default(), &mut app, read_refused);
    let ThingsState::Failed(text) = state(&app) else {
        panic!("the read failed");
    };
    assert_eq!(text, ThingsError::Request.message());
}

#[test]
fn offline_reads_are_answered_disabled_without_a_request() {
    let mut app = app_with(ThingsState::Queued(settings()));
    let mut jobs = ThingsJobs::default();
    for _ in 0..5000 {
        jobs.pump(&mut app);
        if !waiting(&app) {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let ThingsState::Failed(text) = state(&app) else {
        panic!("the read was refused");
    };
    assert_eq!(text, ThingsError::Disabled.message());
}

#[test]
fn a_reply_for_a_dialog_that_was_replaced_is_dropped() {
    let mut app = app_with(ThingsState::Queued(settings()));
    let mut jobs = ThingsJobs::default();
    jobs.pump_with(&mut app, read_refused);
    app.set_overlay(Overlay::Things(Box::new(ThingsDialog::new(
        ThingsState::Queued(settings()),
    ))));
    settle(&mut jobs, &mut app, read_ok);
    assert!(
        matches!(state(&app), ThingsState::Ready(_)),
        "the refused reply reached the new dialog"
    );
}
