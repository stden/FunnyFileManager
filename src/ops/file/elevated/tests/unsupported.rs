//! Purpose: Verify unsupported administrator requests report failure without modifying files.

use super::super::*;
use super::{app_in, delete_of};
use std::ffi::OsString;
use tempfile::TempDir;

#[test]
fn the_helper_flag_reports_unsupported_on_platforms_without_elevation() {
    let arguments = [
        OsString::from(HELPER_FLAG),
        OsString::from("fixture-pipe"),
        OsString::from("1"),
    ];
    let error = helper_main(&arguments)
        .expect("the flag is recognized")
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Unsupported);
    assert!(!available());
    assert!(!session_active());
}

#[test]
fn an_unsupported_retry_closes_the_confirmation_and_preserves_the_source() {
    let root = TempDir::new().unwrap();
    let source = root.path().join("source.txt");
    std::fs::write(&source, "keep").unwrap();
    let retry = delete_of(vec![source.clone()], root.path());
    let failed = FailedEntry::new(&source, &"permission denied");
    let failure = OperationFailed::new(Operation::Delete, failed, 0, 1).offering(Some(retry));
    let mut app = app_in(root.path());
    app.set_overlay(Overlay::OperationFailed(Box::new(failure)));
    retry_as_administrator(&mut app);
    assert!(matches!(app.overlay, Overlay::None));
    assert!(
        matches!(&app.status, Status::Error(message) if message.starts_with("Could not run as administrator:"))
    );
    assert_eq!(std::fs::read_to_string(&source).unwrap(), "keep");
    assert!(app
        .left_panel
        .entries
        .iter()
        .any(|entry| entry.name.as_str() == "source.txt"));
}
