//! Unit tests: the dialog opens on the settings it is given, a read's answer
//! lands in the dialog that asked, and an import saves what was chosen through
//! the store, keeping the entries made by hand. The tests are in `reading` and
//! `importing`; this file holds what they share.

mod importing;
mod reading;

use std::sync::Arc;

use tempfile::TempDir;

use super::{open_with, publish};
use crate::models::things::ThingsState;
use crate::models::{App, Overlay, Session};
use crate::test_support::netbox::{app_on_sessions, file_store};
use crate::things::{Config, ConfigError, Listing, Machine};

pub(super) fn settings() -> Result<Config, ConfigError> {
    Config::from_values(
        Some("https://things.example.com".to_owned()),
        Some("test-token-123".to_owned()),
    )
}

pub(super) fn machine(name: &str, host: &str) -> Machine {
    Machine {
        id: format!("id-{name}"),
        name: name.to_owned(),
        hostname: host.to_owned(),
        ..Machine::default()
    }
}

pub(super) fn two_machines() -> Vec<Machine> {
    vec![
        machine("web-01", "web-01.example.com"),
        machine("db-01", "db-01.example.com"),
    ]
}

/// An app whose store is saved in `dir` and already holds `stored`.
pub(super) fn app(dir: &TempDir, stored: Vec<Session>) -> App {
    app_on_sessions(dir.path(), file_store(dir.path(), stored))
}

/// The open dialog's read, answered with `machines`.
pub(super) fn loaded(app: &mut App, machines: Vec<Machine>, total: Option<usize>) {
    open_with(app, settings());
    let generation = match &app.overlay {
        Overlay::Things(dialog) => Arc::clone(&dialog.generation),
        _ => panic!("the dialog is open"),
    };
    publish(app, &generation, Ok(Listing::new(machines, total)));
}

/// The console transcript's lines, oldest first.
pub(super) fn console_lines(app: &App) -> Vec<String> {
    app.console
        .visible(100)
        .iter()
        .map(|line| line.text().to_owned())
        .collect()
}

pub(super) fn state(app: &App) -> &ThingsState {
    match &app.overlay {
        Overlay::Things(dialog) => &dialog.state,
        _ => panic!("the dialog is open"),
    }
}
