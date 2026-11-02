use super::*;
use crate::models::remote_work::FileLocation;

#[test]
fn a_malformed_open_result_reports_an_error_without_opening_an_editor() {
    let (_dir, mut app, link, _) = host();
    completion::apply(
        &mut app,
        request(&link),
        Ok(RemoteOutcome::Opened {
            source: FileLocation::Local(std::path::PathBuf::new()),
            bytes: b"text".to_vec(),
            edit: true,
        }),
    );
    assert!(!app.overlay.is_active());
    assert!(matches!(app.status, Status::Error(_)));
}

#[test]
fn refreshing_deleted_local_panels_recovers_to_an_existing_parent() {
    let directory = tempfile::TempDir::new().expect("temporary directory");
    let mut app = crate::test_support::netbox::app_in(directory.path());
    std::fs::remove_dir(directory.path()).expect("remove empty panel directory");
    completion::refresh(&mut app);
    assert!(app.left_panel.path.is_dir());
    assert!(app.right_panel.path.is_dir());
    assert_ne!(app.left_panel.path, directory.path());
    assert!(app.remote_requests.is_idle());
}
