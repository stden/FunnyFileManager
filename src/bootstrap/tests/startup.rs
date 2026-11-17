//! Purpose: Start-up composition: a launch folder that is gone falls back to a
//! known one, and a replayed session gets none of what only live input gets.

use std::ffi::OsString;
use std::fs;
use std::io;

use tempfile::TempDir;

use super::super::{live_input, load_app_from};
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
