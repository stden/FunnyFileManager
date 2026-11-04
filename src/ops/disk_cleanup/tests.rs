//! Purpose: Cleanup regressions and outcomes using disposable filesystem fixtures.

use std::fs;

use tempfile::TempDir;

use crate::models::{FileSize, Overlay};
use crate::test_support::disk_cleanup::{app, candidate, dialog, heap_dumps, run};

mod deletion;
mod measurement;
mod temp_folder;

#[test]
fn cancelling_preserves_selected_files_and_closes_the_overlay() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("keep.txt");
    fs::write(&file, b"keep").unwrap();
    let mut app = app(temp.path(), vec![candidate("Cache file", &file)]);

    super::cancel(&mut app);

    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(app.status.to_string(), "Cleanup cancelled");
    assert_eq!(fs::read(file).unwrap(), b"keep");
}

#[test]
fn running_without_a_cleanup_overlay_preserves_state() {
    let temp = TempDir::new().unwrap();
    let mut app = app(temp.path(), Vec::new());
    app.set_overlay(Overlay::Help);
    let before = app.status.to_string();

    run(&mut app);

    assert!(matches!(app.overlay, Overlay::Help));
    assert_eq!(app.status.to_string(), before);
    assert_eq!(super::current_field(&app), None);
}

#[test]
fn an_empty_selection_closes_without_deleting_files() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("keep.txt");
    fs::write(&file, b"keep").unwrap();
    let mut row = candidate("Cache file", &file);
    row.toggle_checked();
    let mut app = app(temp.path(), vec![row]);

    run(&mut app);

    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(app.status.to_string(), "Nothing selected for cleanup");
    assert_eq!(fs::read(file).unwrap(), b"keep");
}

#[test]
fn cleanup_reports_failure_when_contents_candidate_is_not_a_directory() {
    use crate::models::{CandidateState, CleanupCandidate, CleanupKind};
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("file.txt");
    fs::write(&file, b"data").unwrap();
    let rows = vec![CleanupCandidate {
        label: "Not a dir",
        path: file,
        kind: CleanupKind::Contents,
        state: CandidateState::Present {
            size: Some(FileSize::from_bytes(4)),
            checked: true,
        },
    }];
    assert_eq!(super::files::cleanup(&rows).failures, 1);
}

#[test]
fn remove_owned_counts_dir_entry_errors_and_foreign_entries() {
    let mut outcome = super::files::CleanupOutcome::default();
    assert!(!outcome.remove_owned(Err(std::io::Error::other("entry read error"))));
    assert_eq!(outcome.failures, 1);
    #[cfg(unix)]
    if rustix::process::geteuid().as_raw() != 0 {
        use std::os::unix::fs::MetadataExt;
        if let Ok(mut dir) = fs::read_dir("/etc") {
            if let Some(Ok(entry)) = dir.find(|e| {
                e.as_ref()
                    .ok()
                    .and_then(|d| d.metadata().ok())
                    .is_some_and(|m| m.uid() != rustix::process::geteuid().as_raw())
            }) {
                assert!(!outcome.remove_owned(Ok(entry)));
            }
        }
    }
}

#[test]
fn measure_and_cleanup_handle_symlink_targets() {
    use crate::models::{CandidateState, CleanupCandidate, CleanupKind};
    let temp = TempDir::new().unwrap();
    let target = temp.path().join("target_dir");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("file.txt"), b"data").unwrap();
    let link = temp.path().join("link");
    crate::test_support::make_dir_link(&target, &link).unwrap();
    let shared = temp.path().join("shared");
    fs::create_dir(&shared).unwrap();
    crate::test_support::make_dir_link(&target, &shared.join("inner")).unwrap();
    let item = |label, path| CleanupCandidate {
        label,
        path,
        kind: CleanupKind::Contents,
        state: CandidateState::Absent,
    };
    let mut dlg = dialog(vec![item("Link", link.clone()), item("Shared", shared)]);
    super::measure(&mut dlg, temp.path());
    dlg.candidates.push(candidate("Link path", &link));
    assert_eq!(super::files::cleanup(&dlg.candidates).failures, 0);
    assert!(target.join("file.txt").is_file());
}
