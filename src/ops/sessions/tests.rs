//! Unit tests: the sessions panel opens on the side asked for and leaves on
//! `..`; every change to the store is saved before it shows, and a change
//! the file does not take leaves the store as it was.

use std::fs;
use std::io;

use tempfile::TempDir;

use super::{
    back_to_files, back_to_files_on, create, delete, duplicate, edit, execute, install, open,
    open_on, refresh, report, save_edit, view, SessionError,
};
use crate::command::SessionsCommand;
use crate::models::{
    Auth, EditPurpose, Field, InvalidField, KeyFile, Origin, Overlay, PanelKind, PanelSide,
    Problem, RemotePath, RowMotion, SessionEdit,
};
use crate::sessions::import::ImportError;
use crate::sessions::{SessionStore, StoreError, FILE_NAME};
use crate::status::Status;
use crate::test_support::netbox::{
    app_in, app_on_sessions, file_store, name, open_empty_sessions, session,
};

/// The status line for a sessions file holding `{`, which does not parse:
/// the reason is on it, not only in the log.
const DAMAGED: &str = "NetBox: sessions.json did not load (EOF while parsing an object at line 1 \
                       column 1), so it is not overwritten; fix or move it first";

fn sessions_panel(app: &crate::models::App) -> &crate::models::SessionsPanel {
    let PanelKind::Sessions(panel) = &app.left_panel.kind else {
        panic!("the left panel lists the sessions");
    };
    panel
}

mod details;
mod editing;
mod panel_view;
mod reporting;
