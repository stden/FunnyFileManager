//! Purpose: Start-up composition: a launch folder that is gone falls back to a
//! known one, and a replayed session gets none of what only live input gets.

use std::ffi::OsString;
use std::fs;
use std::io;

use tempfile::TempDir;

use super::super::background::install_task_host;
use super::super::{early_cli, live_input, load_app_from};
use crate::history::Persistence;
use crate::models::token_usage::UsageAccess;
use crate::models::ProgramPause;
use crate::recording::Input;

#[test]
fn startup_uses_a_known_directory_when_the_working_directory_disappears() {
    let root = TempDir::new().expect("root");
    let gone = Err(io::Error::from(io::ErrorKind::NotFound));
    let shell_directory = Some(root.path().to_path_buf());
    let app = load_app_from(gone, shell_directory, None, Persistence::SessionOnly)
        .expect("fallback directory");
    assert_eq!(app.left_panel.path, root.path());
    assert_eq!(app.right_panel.path, root.path());
}

#[test]
fn a_replayed_session_has_no_live_process_or_network_permissions() {
    let root = TempDir::new().expect("root");
    let recording = root.path().join("empty.jsonl");
    let header = r#"{"version":1,"executable":"ffm","directory":".","size":[80,24]}"#;
    fs::write(&recording, header).expect("recording header");
    let args = [OsString::from("--replay"), recording.into_os_string()];
    let (input, _) = Input::from_args(&args, (80, 24)).expect("replay");
    let permissions = live_input(&input);
    assert_eq!(permissions.program_pause, ProgramPause::None);
    assert!(!permissions.elevation);
    assert!(matches!(permissions.usage, UsageAccess::Offline));
}

#[test]
fn early_cli_dispatches_things_patch_and_automation_and_skips_plain_args() {
    let things = early_cli(&[
        OsString::from("--things-patch"),
        OsString::from("--unknown"),
    ]);
    assert!(things.expect("things cli").is_err());

    let automation = early_cli(&[OsString::from("--automation"), OsString::from("--unknown")]);
    assert!(automation.expect("automation cli").is_err());

    assert!(early_cli(&[OsString::from("plain")]).is_none());
}

#[test]
fn missing_task_host_leaves_slot_empty() {
    let root = TempDir::new().expect("root");
    let panel = crate::models::Panel::new(root.path().to_path_buf()).expect("panel");
    let mut app = crate::models::App::with_panels(panel.clone(), panel);
    install_task_host(&mut app, None);
    assert!(app.task_host.host().is_none());
}
