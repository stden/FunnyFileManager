//! Unit tests: an import finds its file — named, relative to a file panel,
//! or under a file panel's cursor — takes what it found into the store, and
//! reports the counts on the status bar and every skip in the transcript.

use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;

use super::{prompt, run, submit, EXTENSION};
use crate::models::{App, InputPurpose, Overlay, PanelKind, SessionsPanel};
use crate::ops::sessions::SessionError;
use crate::sessions::import::ImportError;
use crate::sessions::{SessionStore, StoreError, FILE_NAME, MAX_FILE_BYTES};
use crate::test_support::netbox::{app_in, file_store, name, session};

const EXPORT: &str = "<NetBox><Sessions>\
    <Session name=\"web\"><HostName>web.test</HostName><Password>0A0B</Password></Session>\
    <Session name=\"bad\"><HostName>a b</HostName></Session>\
    </Sessions></NetBox>";

/// A directory holding `demo.netbox`, and an app on it whose cursor is on
/// that file.
fn setup() -> (TempDir, App) {
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join(format!("demo.{EXTENSION}")), EXPORT).expect("write");
    let mut app = app_in(dir.path());
    app.sessions = SessionStore::load_from(dir.path().join(FILE_NAME));
    select(&mut app, "demo.netbox");
    (dir, app)
}

fn select(app: &mut App, name: &str) {
    let index = app
        .left_panel
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .expect("listed");
    app.left_panel.select(Some(index));
}

fn transcript(app: &App) -> Vec<String> {
    app.console
        .visible(100)
        .iter()
        .map(|line| line.text().to_owned())
        .collect()
}

mod reading;
mod storing;
