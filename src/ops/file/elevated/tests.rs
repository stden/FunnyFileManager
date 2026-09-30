//! Purpose: The administrator helper's protocol and work, without Windows:
//! requests and replies round-trip, the helper redoes a delete or a copy and
//! stops where FFM would, and FFM reports what it did.

use std::cell::RefCell;
use std::fs;
use std::io::{self, Cursor, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use tempfile::TempDir;

use super::protocol::{self, Failure, Reply};
use super::*;
use crate::copy::{CopyPrompt, Purpose};
use crate::models::{Denial, Panel};

/// A pipe end that keeps what was written to it.
#[derive(Clone, Default)]
struct Written(Rc<RefCell<Vec<u8>>>);

impl Write for Written {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Written {
    fn replies(&self) -> Vec<Reply> {
        let bytes = self.0.borrow().clone();
        let mut input = Cursor::new(bytes);
        std::iter::from_fn(|| protocol::receive::<Reply>(&mut input).expect("a reply")).collect()
    }
}

fn delete_of(sources: Vec<PathBuf>, base: &Path) -> ElevatedRetry {
    ElevatedRetry {
        operation: Operation::Delete,
        selected: sources.len(),
        sources,
        base: base.to_path_buf(),
        settings: None,
    }
}

fn failure_at(path: &Path, denial: Option<Denial>) -> Failure {
    Failure {
        path: path.to_path_buf(),
        error: "denied".to_owned(),
        denial,
    }
}

/// An app on `dir` in both panels.
fn app_in(dir: &Path) -> App {
    App::with_panels(
        Panel::new(dir.to_path_buf()).unwrap(),
        Panel::new(dir.to_path_buf()).unwrap(),
    )
}

fn request_lines(requests: &[ElevatedRetry]) -> Cursor<Vec<u8>> {
    let mut lines = Vec::new();
    for request in requests {
        protocol::send(&mut lines, request).expect("encoded");
    }
    Cursor::new(lines)
}

#[test]
fn a_request_survives_the_pipe() {
    let request = ElevatedRetry {
        operation: Operation::Copy,
        sources: vec![PathBuf::from("a b"), PathBuf::from("ç")],
        base: PathBuf::from("/base"),
        selected: 3,
        settings: Some(CopyPrompt::for_purpose(
            Purpose::Copy,
            "x",
            Path::new("/to"),
        )),
    };
    let mut input = request_lines(std::slice::from_ref(&request));
    let received = protocol::receive::<ElevatedRetry>(&mut input).unwrap();
    assert_eq!(received, Some(request));
    assert_eq!(
        protocol::receive::<ElevatedRetry>(&mut input).unwrap(),
        None
    );
}

#[test]
fn a_reply_survives_the_pipe() {
    let reply = Reply::Finished(Outcome {
        done: 1,
        left: 2,
        failed: Some(failure_at(
            Path::new("p"),
            Some(Denial::FolderLocked(PathBuf::from("/f"))),
        )),
        ..Outcome::default()
    });
    let mut line = Vec::new();
    protocol::send(&mut line, &reply).unwrap();
    let received = protocol::receive::<Reply>(&mut Cursor::new(line)).unwrap();
    assert_eq!(received, Some(reply));
}

#[test]
fn a_line_that_is_not_a_request_is_an_error() {
    let mut input = Cursor::new(b"not json\n".to_vec());
    assert!(protocol::receive::<ElevatedRetry>(&mut input).is_err());
}

#[test]
fn the_helper_deletes_what_it_is_sent_and_says_so() {
    let dir = TempDir::new().unwrap();
    let folder = dir.path().join("folder");
    fs::create_dir_all(folder.join("sub")).unwrap();
    fs::write(folder.join("sub").join("f.txt"), b"x").unwrap();
    let out = Written::default();

    helper::serve(
        request_lines(&[delete_of(vec![folder.clone()], dir.path())]),
        out.clone(),
    )
    .unwrap();

    assert!(!folder.exists());
    let replies = out.replies();
    assert_eq!(
        replies.last(),
        Some(&Reply::Finished(Outcome {
            done: 1,
            ..Outcome::default()
        }))
    );
}

#[test]
fn the_helper_stops_at_the_first_entry_it_cannot_delete() {
    let dir = TempDir::new().unwrap();
    let kept = dir.path().join("kept.txt");
    fs::write(&kept, b"k").unwrap();
    let gone = dir.path().join("gone.txt");

    let outcome = helper::run(&delete_of(vec![gone.clone(), kept.clone()], dir.path()));

    assert!(kept.exists());
    assert_eq!((outcome.done, outcome.left), (0, 1));
    assert_eq!(outcome.failed.map(|failed| failed.path), Some(gone));
}

#[test]
fn the_helper_copies_as_the_dialog_said() {
    let from = TempDir::new().unwrap();
    let to = TempDir::new().unwrap();
    fs::write(from.path().join("a.txt"), b"a").unwrap();
    let settings = CopyPrompt::for_purpose(Purpose::Copy, "a.txt", to.path());

    let outcome = helper::run(&ElevatedRetry {
        operation: Operation::Copy,
        sources: vec![from.path().join("a.txt")],
        base: from.path().to_path_buf(),
        selected: 1,
        settings: Some(settings),
    });

    assert_eq!(outcome.done, 1);
    assert!(outcome.failed.is_none());
    assert_eq!(fs::read(to.path().join("a.txt")).unwrap(), b"a");
}

#[test]
fn a_transfer_without_its_settings_is_never_read_as_a_delete() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("f.txt");
    fs::write(&file, b"f").unwrap();
    let mut request = delete_of(vec![file.clone()], dir.path());
    request.operation = Operation::Move;

    let outcome = helper::run(&request);

    assert!(file.exists());
    assert_eq!(outcome.left, 1);
    assert!(outcome.failed.is_some());
}

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
