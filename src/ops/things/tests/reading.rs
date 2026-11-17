//! Unit tests: the dialog opens on the settings it is given, a read's answer
//! lands in the dialog that asked for it, and a failed read says why.

use std::sync::Arc;

use tempfile::TempDir;

use super::super::{open_with, publish, submit};
use super::{app, loaded, settings, state, two_machines};
use crate::models::things::ThingsState;
use crate::models::Overlay;
use crate::status::Status;
use crate::things::{ConfigError, Listing, ThingsError, TOKEN_VARIABLE, URL_VARIABLE};

#[test]
fn without_settings_the_dialog_names_the_variables_and_reads_nothing() {
    let dir = TempDir::new().expect("temp");
    let mut app = app(&dir, Vec::new());
    open_with(
        &mut app,
        Err(ConfigError::Missing(vec![URL_VARIABLE, TOKEN_VARIABLE])),
    );
    let ThingsState::Setup(text) = state(&app) else {
        panic!("the dialog says what to set");
    };
    assert!(
        text.contains(URL_VARIABLE) && text.contains(TOKEN_VARIABLE),
        "{text}"
    );
}

#[test]
fn with_settings_the_read_is_queued_for_the_loop() {
    let dir = TempDir::new().expect("temp");
    let mut app = app(&dir, Vec::new());
    open_with(&mut app, settings());
    assert!(matches!(state(&app), ThingsState::Queued(_)));
}

#[test]
fn the_status_line_counts_the_machines_and_the_servers_total() {
    let dir = TempDir::new().expect("temp");
    let mut app = app(&dir, Vec::new());
    loaded(&mut app, two_machines(), Some(120));
    assert!(matches!(state(&app), ThingsState::Ready(_)));
    assert_eq!(
        app.status,
        Status::CommandOutput("Things: showing 2 of 120 machines".to_owned())
    );
}

#[test]
fn a_full_listing_is_counted_without_a_total() {
    let dir = TempDir::new().expect("temp");
    let mut app = app(&dir, Vec::new());
    loaded(&mut app, two_machines(), Some(2));
    assert_eq!(
        app.status,
        Status::CommandOutput("Things: 2 machines".to_owned())
    );
}

#[test]
fn a_reply_to_a_dialog_that_was_reopened_is_dropped() {
    let dir = TempDir::new().expect("temp");
    let mut app = app(&dir, Vec::new());
    open_with(&mut app, settings());
    let stale = match &app.overlay {
        Overlay::Things(dialog) => Arc::clone(&dialog.generation),
        _ => panic!("the dialog is open"),
    };
    open_with(&mut app, settings());
    publish(&mut app, &stale, Ok(Listing::new(two_machines(), None)));
    assert!(matches!(state(&app), ThingsState::Queued(_)));
}

#[test]
fn a_failed_read_fails_the_dialog_with_its_message() {
    let dir = TempDir::new().expect("temp");
    let mut app = app(&dir, Vec::new());
    open_with(&mut app, settings());
    let generation = match &app.overlay {
        Overlay::Things(dialog) => Arc::clone(&dialog.generation),
        _ => panic!("the dialog is open"),
    };
    publish(&mut app, &generation, Err(ThingsError::Request));
    let ThingsState::Failed(text) = state(&app) else {
        panic!("the read failed");
    };
    assert_eq!(text, ThingsError::Request.message());
}

#[test]
fn enter_on_a_dialog_that_is_not_showing_machines_just_closes_it() {
    let dir = TempDir::new().expect("temp");
    let mut app = app(&dir, Vec::new());
    open_with(&mut app, Err(ConfigError::NotHttps));
    submit(&mut app);
    assert!(!app.overlay.is_active());
}
