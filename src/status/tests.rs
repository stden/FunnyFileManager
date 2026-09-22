use super::*;

#[test]
fn console_start_status_reports_launch_without_claiming_completion() {
    let command = crate::command::ShellCommand::new("codex resume").expect("command");
    assert_eq!(
        Status::ConsoleStarted(command).to_string(),
        "Started in new console: codex resume"
    );
}

/// The owned-string conversion uses the same user-facing rendering as `Display`.
#[test]
fn a_status_converts_into_a_plain_string() {
    let text: String = Status::UnmarkedAll.into();
    assert_eq!(text, "Unmarked all");
}

#[test]
fn a_failed_entry_keeps_the_path_and_the_reason_apart() {
    // Joined into one string at the point of failure, they could only ever
    // be shown one way; kept apart, a dialog can show the full path while
    // the status bar shows the summary.
    let failure = FailedEntry::new("/srv/work/notes.txt", &"Permission denied");
    assert_eq!(failure.path(), std::path::Path::new("/srv/work/notes.txt"));
    assert_eq!(failure.error(), "Permission denied");
    assert_eq!(
        failure.to_string(),
        "/srv/work/notes.txt: Permission denied"
    );
}

#[test]
fn a_failed_entry_takes_its_reason_from_any_error() {
    // `&dyn Display` so an `io::Error` needs no formatting at the call
    // site — that formatting is what used to lose the path.
    let error = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
    let failure = FailedEntry::new("/tmp/x", &error);
    assert_eq!(failure.error(), "gone");
}

/// The status bar has exactly one input.
///
/// Two call sites used to assign `App.status_message` directly, bypassing
/// this module — and one bdd step silently wrote to a temporary once the
/// field became a `Status`. Keeping the outcome rather than its rendering
/// is what lets a test match the outcome.
#[test]
fn free_form_output_still_travels_as_a_status() {
    assert_eq!(Status::Cleared.to_string(), "");
    assert_eq!(
        Status::Path(std::path::PathBuf::from("/srv/work")).to_string(),
        "/srv/work"
    );
    assert_eq!(
        Status::CommandOutput("total 0".to_owned()).to_string(),
        "total 0"
    );
    assert!(Status::KeyHints.to_string().contains("F1 Help"));
}

#[test]
fn a_fresh_app_opens_on_the_key_hints() {
    let app = App::new().expect("app");
    assert_eq!(app.status, Status::KeyHints);
    assert!(app.status_text().contains("F5 Copy"));
}

#[test]
fn set_status_keeps_the_outcome_not_its_rendering() {
    let mut app = App::new().expect("app");
    app.set_status(Status::UnmarkedAll);
    assert_eq!(app.status, Status::UnmarkedAll);
}

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

#[test]
fn editor_open_statuses_match_legacy_strings() {
    assert_status_text(&[
        (Status::Editing("/e".into()), "Editing /e"),
        (Status::Edited("/e".into()), "Edited /e"),
        (
            Status::CannotEdit {
                path: PathBuf::from("/e"),
                error: "nope".into(),
            },
            "Cannot edit /e: nope",
        ),
        (
            Status::EditDirUnsupported,
            "Editing directories is not supported",
        ),
        (
            Status::FileTooLargeEdit {
                megabytes: 12.34,
                limit_megabytes: 10.0,
            },
            "File too large to edit (12.3 MB, max 10 MB)",
        ),
    ]);
}

#[test]
fn editor_save_statuses_match_legacy_strings() {
    assert_status_text(&[
        (
            Status::EditorExited {
                status: "exit status: 1".into(),
            },
            "Editor exited with status exit status: 1",
        ),
        (
            Status::LaunchEditorFailed { error: "x".into() },
            "Failed to launch editor: x",
        ),
        (Status::Saved(PathBuf::from("/s")), "Saved /s"),
        (Status::SaveFailed { error: "x".into() }, "Save failed: x"),
        (Status::ChangesDiscarded, "Changes discarded"),
    ]);
}

#[test]
fn new_file_statuses_match_legacy_strings() {
    assert_status_text(&[
        (Status::FilenameEmpty, "Filename cannot be empty"),
        (
            Status::FileExists("a.txt".into()),
            "File already exists: a.txt",
        ),
        (Status::CreatedFile("a.txt".into()), "Created file: a.txt"),
        (
            Status::CreateFileFailed { error: "x".into() },
            "Failed to create file: x",
        ),
    ]);
}

#[test]
fn platform_statuses_match_legacy_strings() {
    assert_status_text(&[
        (
            Status::DeleteFailed { error: "x".into() },
            "Delete failed: x",
        ),
        (Status::DeleteCancelled, "Delete cancelled"),
        (Status::WipeCancelled, "Wipe cancelled"),
        (Status::NothingToWipe, "Nothing to wipe"),
        (
            Status::OpenDriveFailed { error: "x".into() },
            "Failed to open drive: x",
        ),
        (Status::ChangedTo(PathBuf::from("/p")), "Changed to /p"),
        (
            Status::S3ProfileSelected("dev".into()),
            "S3 profile 'dev' selected (S3 browser coming soon)",
        ),
        (Status::Error("x".into()), "Error: x"),
        (
            Status::WindowsOnly { tool: "system" },
            "Windows system checker is only available on Windows",
        ),
    ]);
}

/// The limit in the message must follow the limit that was actually
/// enforced — it used to be hard-coded as "max 10 MB" in the text.
#[test]
fn too_large_messages_report_the_carried_limit() {
    assert_eq!(
        Status::FileTooLarge {
            megabytes: 60.0,
            limit_megabytes: 50.0,
        }
        .to_string(),
        "File too large to view (60.0 MB, max 50 MB)"
    );
    assert_eq!(
        Status::FileTooLargeEdit {
            megabytes: 60.0,
            limit_megabytes: 50.0,
        }
        .to_string(),
        "File too large to edit (60.0 MB, max 50 MB)"
    );
}

/// The limit the handlers pass comes from one constant, so the rendered
/// message and the guard cannot drift apart.
#[test]
fn view_limit_message_matches_the_constant() {
    let limit = crate::utils::MAX_FILE_SIZE.megabytes();
    assert_eq!(
        Status::FileTooLarge {
            megabytes: limit + 1.0,
            limit_megabytes: limit,
        }
        .to_string(),
        "File too large to view (11.0 MB, max 10 MB)"
    );
}

/// A copy outcome names the *field label*, never the copied value — the
/// variant carries only a [`Secret`] discriminant, so the status bar
/// cannot leak what went to the clipboard.
#[test]
fn copied_status_names_the_label_not_the_secret() {
    assert_eq!(
        Status::Copied(Secret::Password).to_string(),
        "Password copied"
    );
    assert_eq!(
        Status::Copied(Secret::Username).to_string(),
        "User name copied"
    );
    assert_eq!(
        Status::CopiedViaOsc52(Secret::Password).to_string(),
        "Password sent to the terminal via OSC 52"
    );
    assert_eq!(
        Status::CopiedViaOsc52(Secret::Username).to_string(),
        "User name sent to the terminal via OSC 52"
    );
}

/// The vault outcomes carry no key material: the only payloads are the
/// file name in `VaultOpened` and the pre-redacted error in `CopyFailed`.
/// `UnlockFailed` is deliberately bare — the reason sits beside the
/// password field, not on the status bar.
#[test]
fn vault_statuses_show_only_the_file_name_and_a_redacted_error() {
    assert_eq!(
        Status::VaultOpened("vault.kdbx".to_owned()).to_string(),
        "Opened vault.kdbx: enter the master password"
    );
    assert_eq!(Status::VaultLocked.to_string(), "Vault locked");
    assert_eq!(
        Status::VaultUnlocked { entries: 5 }.to_string(),
        "Vault unlocked: 5 entries"
    );
    assert_eq!(Status::UnlockFailed.to_string(), "Unlock failed");
    assert_eq!(
        Status::CopyFailed {
            error: "no clipboard available".to_owned(),
        }
        .to_string(),
        "Copy failed: no clipboard available"
    );
}

fn assert_status_text(cases: &[(Status, &str)]) {
    for (status, expected) in cases {
        assert_eq!(&status.to_string(), expected, "mismatch for {status:?}");
    }
}

/// Far's info row carries a message only until the user moves on. Before this,
/// an error sat in place of the selected entry's name for every later frame,
/// until some other operation happened to replace it.
#[test]
fn a_message_survives_its_own_frame_and_goes_when_the_cursor_moves() {
    let temp = tempfile::TempDir::new().expect("temp");
    std::fs::write(temp.path().join("first.txt"), "").expect("first");
    std::fs::write(temp.path().join("second.txt"), "").expect("second");
    let mut app = App::new().expect("app");
    app.left_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");
    app.right_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");

    app.set_status(Status::Error("Permission denied".to_owned()));
    assert!(
        app.status_news().is_some(),
        "the message must exist to start"
    );

    // The frame that shows it. A message set since the last frame is tied to
    // the cursor this one draws, so it cannot be cleared before it is seen.
    app.expire_status_on_move();
    assert!(
        app.status_news().is_some(),
        "the message was cleared before it reached the screen"
    );

    // A second frame with the cursor unmoved still shows it: only moving away
    // ends it, not the mere passage of frames.
    app.expire_status_on_move();
    assert!(
        app.status_news().is_some(),
        "an idle frame ended the message"
    );

    app.get_active_panel().next();
    app.expire_status_on_move();
    assert!(
        app.status_news().is_none(),
        "the entry name must come back once the cursor moves"
    );
}

/// Switching panels is moving on too: the other panel's row is a different
/// place to be looking, so the message belongs to neither any more.
#[test]
fn a_message_does_not_follow_the_cursor_to_the_other_panel() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = App::new().expect("app");
    app.left_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");
    app.right_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");

    app.set_status(Status::Error("Permission denied".to_owned()));
    app.expire_status_on_move();
    app.switch_panel();
    app.expire_status_on_move();

    assert!(app.status_news().is_none());
}

/// The info row and the command prompt read the same field. Expiring a message
/// is a presentation decision for the row only: a command that failed must not
/// start reporting success because the cursor moved.
#[test]
fn expiry_does_not_rewrite_the_prompts_verdict() {
    use crate::prompt::LastCommand;

    let temp = tempfile::TempDir::new().expect("temp");
    std::fs::write(temp.path().join("first.txt"), "").expect("first");
    std::fs::write(temp.path().join("second.txt"), "").expect("second");
    let mut app = App::new().expect("app");
    app.left_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");
    app.right_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");

    app.set_status(Status::SaveFailed {
        error: "permission denied".to_owned(),
    });
    app.expire_status_on_move();
    app.get_active_panel().next();
    app.expire_status_on_move();

    assert!(app.status_news().is_none(), "the row gave the name back");
    assert_eq!(
        LastCommand::from_status(&app.status),
        LastCommand::Failed,
        "the prompt forgot that the last command failed"
    );
}

/// A quick search owns the info row while it is running, so a message raised
/// underneath it has not been seen. Expiring it there would throw away the
/// result of an F5 or a Ctrl+R the user never got to read.
#[test]
fn a_message_raised_under_a_quick_search_is_not_thrown_away() {
    let temp = tempfile::TempDir::new().expect("temp");
    std::fs::write(temp.path().join("first.txt"), "").expect("first");
    std::fs::write(temp.path().join("second.txt"), "").expect("second");
    let mut app = App::new().expect("app");
    app.left_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");
    app.right_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");

    app.quick_search.push('f');
    app.set_status(Status::PanelsRefreshed);
    app.expire_status_on_move();
    app.get_active_panel().next();
    app.expire_status_on_move();

    app.quick_search.clear();
    app.expire_status_on_move();
    assert!(
        app.status_news().is_some(),
        "the refresh result was discarded while the search hid it"
    );
}
