//! Purpose: the rendered text of run, delete, attribute, panel and bookmark outcomes, locked to their historical strings.

use super::*;

/// How a program started with Enter ended, in the words the transcript and
/// the status bar share.
#[test]
fn run_statuses_match_legacy_strings() {
    let status = std::process::ExitStatus::default();
    assert_status_text(&[
        (
            Status::ProgramExited {
                program: "/x/run.sh".into(),
                status,
            },
            &format!("/x/run.sh: {status}"),
        ),
        (
            Status::CannotRun {
                program: "/x/run.sh".into(),
                error: "denied".into(),
            },
            "Error: Cannot run /x/run.sh: denied",
        ),
    ]);
}

#[test]
fn delete_statuses_match_legacy_strings() {
    assert_status_text(&[
        (Status::Deleted { count: 3 }, "Deleted 3 item(s)"),
        (
            Status::DeletedWithErrors {
                count: 2,
                error_count: 1,
                first_error: FailedEntry::new("/srv/x", &"boom"),
            },
            "Deleted 2 item(s), 1 error(s): /srv/x: boom",
        ),
        (Status::Wiped { count: 3 }, "Wiped 3 item(s)"),
        (
            Status::WipedSharedFiles {
                count: 3,
                shared: 1,
            },
            "Wiped 3 item(s); 1 hard-linked file(s) only unlinked, \
             their data stays under the other names",
        ),
        (
            Status::WipedWithErrors {
                count: 2,
                error_count: 1,
                first_error: FailedEntry::new("/srv/x", &"boom"),
            },
            "Wiped 2 item(s), 1 error(s): /srv/x: boom",
        ),
    ]);
}

#[test]
fn attribute_statuses_match_legacy_strings() {
    assert_status_text(&[
        (
            Status::AttributesSet { count: 2 },
            "Attributes set on 2 object(s)",
        ),
        (
            Status::AttributesSetWithErrors {
                count: 1,
                error_count: 1,
                first_error: FailedEntry::new("/srv/x", &"boom"),
            },
            "Attributes set on 1 object(s), 1 failed: /srv/x: boom",
        ),
    ]);
}

#[test]
fn panel_statuses_match_legacy_strings() {
    assert_status_text(&[
        (Status::Opened(PathBuf::from("/p")), "Opened /p"),
        (Status::PanelsSwapped, "Panels swapped"),
        (Status::PanelsRefreshed, "Panels refreshed"),
        (Status::RefreshFailed("io".into()), "Refresh failed: io"),
        (Status::Sort("Name".into()), "Sort: Name"),
        (Status::Found("q".into()), "Found: q"),
        (Status::NotFound("q".into()), "Not found: q"),
        (Status::Marked { count: 4 }, "Marked 4 items"),
        (Status::UnmarkedAll, "Unmarked all"),
    ]);
}

#[test]
fn bookmark_success_statuses_match_legacy_strings() {
    assert_status_text(&[
        (
            Status::BookmarkSet {
                slot: BookmarkSlot::new(2).expect("legal slot"),
                path: PathBuf::from("/b"),
            },
            "Bookmark 2 set to /b",
        ),
        (
            Status::OpenedBookmark {
                slot: BookmarkSlot::new(2).expect("legal slot"),
                path: PathBuf::from("/b"),
            },
            "Opened bookmark 2: /b",
        ),
    ]);
}

#[test]
fn bookmark_failure_statuses_match_legacy_strings() {
    assert_status_text(&[
        (
            Status::OpenBookmarkFailed {
                slot: BookmarkSlot::new(2).expect("legal slot"),
                error: "io".into(),
            },
            "Failed to open bookmark 2: io",
        ),
        (
            Status::BookmarkEmpty {
                slot: BookmarkSlot::new(5).expect("legal slot"),
            },
            "Bookmark 5 is empty",
        ),
        (
            Status::InvalidBookmarkSlot { slot: "9".into() },
            "Invalid bookmark slot 9",
        ),
    ]);
}
