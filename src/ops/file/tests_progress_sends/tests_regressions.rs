//! Purpose: Real transfers report their final destination and actual file size.

use super::*;
use crate::copy::ExistingFiles;
use crate::progress::FileProgress;

#[test]
fn overwriting_reports_final_destination_and_file_size() {
    let _serial = RENAME_TEST_MUTEX
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let (recorder, _installed) = Recorder::install();
    let temp = TempDir::new().expect("test setup");
    let (from, to) = one_file_to_move(&temp);
    fs::write(to.join("a.txt"), b"old").expect("test setup");
    let mut app = app_on(&from, "a.txt", &to);
    let mut settings = CopyPrompt::for_purpose(Purpose::Copy, "a.txt", &to);
    settings.existing = ExistingFiles::Overwrite;

    crate::ops::copy_with(&mut app, &settings);

    let frames = recorder.frames();
    assert!(
        frames.iter().any(|frame| matches!(frame.transfer.as_ref(),
            Some(transfer) if transfer.destination == to.join("a.txt")
                && matches!(transfer.file, FileProgress::Copying { size: 3, .. })
        )),
        "the destination and size must name the final copy"
    );
    assert!(
        frames
            .iter()
            .filter_map(|frame| frame.transfer.as_ref())
            .all(|transfer| transfer.destination == to.join("a.txt")),
        "staging names must not be shown"
    );
}

#[test]
fn a_failed_copy_does_not_mark_the_file_complete() {
    let _serial = RENAME_TEST_MUTEX
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let (recorder, _installed) = Recorder::install();
    let temp = TempDir::new().expect("test setup");
    let (from, to) = one_file_to_move(&temp);
    fs::create_dir(to.join("a.txt")).expect("test setup");
    let mut app = app_on(&from, "a.txt", &to);
    let mut settings = CopyPrompt::for_purpose(Purpose::Copy, "a.txt", &to);
    settings.existing = ExistingFiles::Overwrite;

    crate::ops::copy_with(&mut app, &settings);

    assert!(
        recorder.frames().iter().all(|frame| !matches!(
            frame.transfer.as_ref().map(|transfer| transfer.file),
            Some(FileProgress::Done)
        )),
        "failed transfers must not report completion"
    );
}

#[test]
fn skipped_files_are_not_reported_as_copied() {
    let _serial = RENAME_TEST_MUTEX
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    for policy in [ExistingFiles::Skip, ExistingFiles::OnlyNewer] {
        let (recorder, _installed) = Recorder::install();
        let temp = TempDir::new().expect("test setup");
        let (from, to) = one_file_to_move(&temp);
        fs::write(to.join("a.txt"), b"old").expect("destination");
        let source = fs::OpenOptions::new()
            .write(true)
            .open(from.join("a.txt"))
            .expect("source");
        let earlier = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
        source
            .set_times(fs::FileTimes::new().set_modified(earlier))
            .expect("source time");
        let mut app = app_on(&from, "a.txt", &to);
        let mut settings = CopyPrompt::for_purpose(Purpose::Copy, "a.txt", &to);
        settings.existing = policy;

        crate::ops::copy_with(&mut app, &settings);

        assert_eq!(fs::read(to.join("a.txt")).expect("destination"), b"old");
        assert!(
            recorder.frames().iter().all(|frame| !matches!(
                frame.transfer.as_ref().map(|transfer| transfer.file),
                Some(FileProgress::Done)
            )),
            "skipped files are not completed: {policy:?}"
        );
    }
}

#[cfg(unix)]
#[test]
fn staged_symlink_overwrite_reports_the_final_destination() {
    use std::os::unix::fs::symlink;

    let _serial = RENAME_TEST_MUTEX
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let (recorder, _installed) = Recorder::install();
    let temp = TempDir::new().expect("test setup");
    let (from, to) = one_file_to_move(&temp);
    fs::remove_file(from.join("a.txt")).expect("test setup");
    symlink("source-target", from.join("a.txt")).expect("source link");
    symlink("old-target", to.join("a.txt")).expect("destination link");
    let mut app = app_on(&from, "a.txt", &to);
    let mut settings = CopyPrompt::for_purpose(Purpose::Copy, "a.txt", &to);
    settings.existing = ExistingFiles::Overwrite;

    crate::ops::copy_with(&mut app, &settings);

    assert_eq!(
        fs::read_link(to.join("a.txt")).expect("copied"),
        PathBuf::from("source-target")
    );
    assert!(
        recorder
            .frames()
            .iter()
            .filter_map(|frame| frame.transfer.as_ref())
            .all(|transfer| transfer.destination == to.join("a.txt")),
        "staging names must not be shown"
    );
}
