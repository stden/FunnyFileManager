//! Purpose: Requests and replies round-trip, and the helper redoes a delete or a copy.

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use super::super::protocol::{self, Reply};
use super::super::*;
use super::{delete_of, failure_at, request_lines, Written};
use crate::copy::{CopyPrompt, Purpose};
use crate::models::Denial;

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

fn serve_once(request: &protocol::Request) -> Vec<Reply> {
    let (mut wire, out) = (Vec::new(), Written::default());
    protocol::send(&mut wire, request).unwrap();
    helper::serve(Cursor::new(wire), out.clone()).unwrap();
    out.replies()
}

#[test]
fn the_helper_honours_per_file_clash_choices() {
    use crate::copy::ExistingFiles::{Ask, Overwrite, Skip};
    let (from, to) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let (a, b) = (from.path().join("a.txt"), from.path().join("b.txt"));
    let (ta, tb) = (to.path().join("a.txt"), to.path().join("b.txt"));
    for (p, d) in [(&a, "new a"), (&b, "new b"), (&ta, "old a"), (&tb, "old b")] {
        fs::write(p, d).unwrap();
    }
    let mut settings = CopyPrompt::for_purpose(Purpose::Copy, "marked", to.path());
    settings.existing = Ask;
    let request = protocol::Request {
        retry: ElevatedRetry {
            operation: Operation::Copy,
            sources: vec![a.clone(), b.clone()],
            base: from.path().to_path_buf(),
            selected: 2,
            settings: Some(settings),
        },
        choices: vec![(a, Overwrite), (b, Skip)],
    };
    let replies = serve_once(&request);
    assert_eq!(fs::read_to_string(&ta).unwrap(), "new a");
    assert_eq!(fs::read_to_string(&tb).unwrap(), "old b");
    let finished = Reply::Finished(Outcome {
        done: 1,
        skipped: 1,
        ..Outcome::default()
    });
    assert_eq!(replies.last(), Some(&finished));
}
