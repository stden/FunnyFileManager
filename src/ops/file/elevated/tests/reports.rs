//! Purpose: Which refusals are offered to an administrator, and how the outcome is shown.

use std::path::PathBuf;

use tempfile::TempDir;

use super::super::*;
use super::{app_in, delete_of, failure_at};
use crate::copy::{CopyPrompt, Purpose};
use crate::models::Denial;

#[test]
fn only_a_refusal_an_administrator_may_get_past_is_offered() {
    let dir = TempDir::new().unwrap();
    let remaining = [dir.path().join("a")];
    let offer = |denial: Option<&Denial>| {
        retry_for(
            true,
            Operation::Delete,
            denial,
            &remaining,
            dir.path(),
            1,
            None,
        )
    };
    assert_eq!(offer(None), None);
    assert_eq!(offer(Some(&Denial::InUse)), None);
    assert_eq!(offer(Some(&Denial::ReadOnly)), None);
    assert_eq!(offer(Some(&Denial::NotEvenAdministrator)), None);
    let offered = offer(Some(&Denial::NeedsAdministrator));
    assert_eq!(offered.is_some(), available());
    if let Some(retry) = offered {
        assert_eq!(retry.sources, remaining);
        assert_eq!(retry.selected, 1);
    }
}

#[test]
fn only_a_live_session_asks_windows() {
    let dir = TempDir::new().unwrap();
    let remaining = [dir.path().join("a")];
    let denial = Some(&Denial::NeedsAdministrator);
    let not_live = retry_for(
        false,
        Operation::Delete,
        denial,
        &remaining,
        dir.path(),
        1,
        None,
    );
    assert_eq!(not_live, None);
}

#[test]
fn what_the_administrator_did_is_added_to_what_was_done_before() {
    let dir = TempDir::new().unwrap();
    let mut app = app_in(dir.path());
    let retry = delete_of(vec![dir.path().join("a"), dir.path().join("b")], dir.path());
    let done = Outcome {
        done: 2,
        ..Outcome::default()
    };
    finish(&mut app, 3, &retry, &done);
    assert!(app.status.to_string().contains('5'), "{}", app.status);
    assert!(!app.overlay.is_active());
}

#[test]
fn what_the_administrator_could_not_do_is_shown_with_nothing_more_to_offer() {
    let dir = TempDir::new().unwrap();
    let mut app = app_in(dir.path());
    let retry = delete_of(vec![dir.path().join("a"), dir.path().join("b")], dir.path());
    let failed = Outcome {
        done: 1,
        failed: Some(failure_at(
            &dir.path().join("b"),
            Some(Denial::NotEvenAdministrator),
        )),
        ..Outcome::default()
    };
    finish(&mut app, 3, &retry, &failed);
    let Overlay::OperationFailed(failure) = &app.overlay else {
        panic!("the administrator's failure is shown");
    };
    assert_eq!(failure.done(), 4);
    assert!(failure.retry().is_none(), "nothing more to offer");
    assert_eq!(failure.denial(), Some(&Denial::NotEvenAdministrator));
}

#[test]
fn a_transfer_the_administrator_finished_reads_as_one() {
    let dir = TempDir::new().unwrap();
    let mut app = app_in(dir.path());
    for purpose in [Purpose::Copy, Purpose::Move] {
        app.close_overlay();
        let retry = ElevatedRetry {
            operation: Operation::of(purpose),
            sources: vec![dir.path().join("a")],
            base: dir.path().to_path_buf(),
            selected: 1,
            settings: Some(CopyPrompt::for_purpose(purpose, "a", dir.path())),
        };
        let done = Outcome {
            done: 1,
            ..Outcome::default()
        };
        finish(&mut app, 0, &retry, &done);
        assert!(!app.overlay.is_active());
        let failed = Outcome {
            failed: Some(failure_at(&dir.path().join("a"), None)),
            ..Outcome::default()
        };
        finish(&mut app, 0, &retry, &failed);
        assert!(
            app.status.to_string().contains("1 error(s)"),
            "{}",
            app.status
        );
    }
}

#[test]
fn other_command_lines_do_not_start_the_helper() {
    use std::ffi::OsString;

    assert!(helper_main(&[]).is_none());
    assert!(helper_main(&[OsString::from("--replay")]).is_none());
    let not_the_flag = [
        OsString::from("--other"),
        OsString::from("n"),
        OsString::from("1"),
    ];
    assert!(helper_main(&not_the_flag).is_none());
}

#[test]
fn every_denial_explains_itself() {
    for denial in [
        Denial::InUse,
        Denial::ReadOnly,
        Denial::NeedsAdministrator,
        Denial::NotEvenAdministrator,
        Denial::FolderLocked(PathBuf::from("/locked")),
        Denial::OtherOwner,
        Denial::Rights,
    ] {
        assert!(!denial.explanation().is_empty());
    }
    assert!(Denial::FolderLocked(PathBuf::from("/locked"))
        .explanation()
        .contains("/locked"));
}
