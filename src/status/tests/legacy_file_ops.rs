//! Purpose: the rendered text of view, copy, move, archive and name outcomes, locked to their historical strings.

use super::*;

/// Locks every variant's rendered text to its historical `format!` string,
/// so the migration away from stringly-typed messages cannot drift.
#[test]
fn view_statuses_match_legacy_strings() {
    assert_status_text(&[
        (
            Status::FileTooLarge {
                megabytes: 12.34,
                limit_megabytes: 10.0,
            },
            "File too large to view (12.3 MB, max 10 MB)",
        ),
        (
            Status::BinaryFile("/bin/ls".into()),
            "Cannot view binary file /bin/ls",
        ),
        (
            Status::NotRegularFile("/dev/zero".into()),
            "Cannot open /dev/zero: not a regular file",
        ),
        (Status::Viewing("/x".into()), "Viewing /x"),
        (Status::CreatedDir("/d".into()), "Created directory /d"),
    ]);
}

#[test]
fn copy_move_statuses_match_legacy_strings() {
    assert_status_text(&[
        (
            Status::CopiedWithErrors {
                count: 2,
                error_count: 1,
                first_error: FailedEntry::new("/srv/x", &"boom"),
            },
            "Copied 2 item(s), 1 error(s): /srv/x: boom",
        ),
        (
            Status::MovedWithErrors {
                count: 2,
                error_count: 1,
                first_error: FailedEntry::new("/srv/x", &"boom"),
            },
            "Moved 2 item(s), 1 error(s): /srv/x: boom",
        ),
    ]);
}

#[test]
fn transfer_statuses_match_legacy_strings() {
    let at = |path: &str| Landed::At(PathBuf::from(path));
    assert_status_text(&[
        (
            transferred(Purpose::Copy, at("/d/a.txt"), 0, 0),
            "Copied to /d/a.txt",
        ),
        (
            transferred(Purpose::Move, Landed::Many(3), 0, 0),
            "Moved 3 item(s)",
        ),
        (
            transferred(Purpose::Copy, Landed::Many(2), 1, 1),
            "Copied 2 item(s), 1 skipped (existing files kept, new ones renamed)",
        ),
        (
            transferred(Purpose::Move, at("/d/a_1.txt"), 0, 1),
            "Moved to /d/a_1.txt (existing files kept, new ones renamed)",
        ),
        (
            transferred(Purpose::Copy, Landed::Nowhere, 2, 0),
            "Copied nothing, 2 skipped",
        ),
    ]);
}

/// The status line names where one entry went, and counts several.
#[test]
fn a_transfer_names_one_destination_and_counts_several() {
    let first = || Some(PathBuf::from("/d/a.txt"));
    assert_eq!(
        Landed::of(1, first()),
        Landed::At(PathBuf::from("/d/a.txt"))
    );
    assert_eq!(Landed::of(2, first()), Landed::Many(2));
    assert_eq!(Landed::of(0, None), Landed::Nowhere);
}

#[test]
fn archive_statuses_match_legacy_strings() {
    let extracted = |count| Status::Extracted {
        count,
        destination: "/d".into(),
    };
    let failed = |done, total| Status::ExtractedWithErrors {
        done,
        total,
        first_error: "bad".into(),
    };
    let added = |kept| Status::AddedToArchive {
        count: 2,
        archive: "/a.tar".into(),
        kept,
    };
    assert_status_text(&[
        (extracted(1), "Extracted 1 archive to /d"),
        (extracted(2), "Extracted 2 archives to /d"),
        (failed(0, 1), "Extracted 0 of 1 archive, first error: bad"),
        (failed(1, 2), "Extracted 1 of 2 archives, first error: bad"),
        (added(0), "Added 2 item(s) to /a.tar"),
        (
            added(1),
            "Added 2 item(s) to /a.tar; 1 could not be deleted",
        ),
    ]);
}

#[test]
fn name_statuses_match_legacy_strings() {
    assert_status_text(&[
        (
            Status::CreatedDirs {
                count: 3,
                directory: "/d".into(),
            },
            "Created 3 folders in /d",
        ),
        (
            Status::AlreadyExists("notes.txt".into()),
            "notes.txt already exists",
        ),
        (
            Status::Linked {
                link: "/d/l".into(),
                target: "../t".into(),
            },
            "Link l -> ../t",
        ),
    ]);
}
