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
}

/// A partly failed batch quotes the path it failed on between its summary and
/// the reason, so the info row can shorten the path and keep both.
#[test]
fn a_failure_quotes_its_path_between_the_summary_and_the_reason() {
    let status = Status::DeletedWithErrors {
        count: 2,
        error_count: 1,
        first_error: FailedEntry::new("/srv/work/notes.txt", &"Permission denied"),
    };
    let failure = status.path_message().expect("quotes a path");
    assert_eq!(
        (
            failure.before.as_ref(),
            failure.path.as_ref(),
            failure.after.as_ref()
        ),
        (
            "Deleted 2 item(s), 1 error(s): ",
            "/srv/work/notes.txt",
            ": Permission denied"
        )
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
        Status::Path(PathBuf::from("/srv/work")).to_string(),
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

/// A copy or move that went through, `skipped` and `renamed` by the F5 / F6
/// dialog's "already existing files" choice.
fn transferred(purpose: Purpose, landed: Landed, skipped: usize, renamed: usize) -> Status {
    Status::Transferred {
        purpose,
        landed,
        skipped,
        renamed,
    }
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
        (
            Status::SetupSaved(PathBuf::from("/c/settings.yaml")),
            "Setup saved to /c/settings.yaml",
        ),
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

/// One of every viewer, editor and program-run outcome that quotes a path,
/// each under `/srv/x/`.
fn view_edit_and_run_statuses_quoting_a_path() -> Vec<Status> {
    let path = PathBuf::from("/srv/x/perms/unreadable.txt");
    vec![
        Status::CannotEdit {
            path: path.clone(),
            error: "Permission denied (os error 13)".to_owned(),
        },
        Status::NotRegularFile("/srv/x/fifo".to_owned()),
        Status::BinaryFile("/srv/x/a.bin".to_owned()),
        Status::Viewing("/srv/x/a.txt".to_owned()),
        Status::Saved(path.clone()),
        Status::Opened(path),
        Status::ProgramExited {
            program: "/srv/x/run.sh".into(),
            status: std::process::ExitStatus::default(),
        },
        Status::CannotRun {
            program: "/srv/x/run.sh".into(),
            error: "denied".into(),
        },
    ]
}

/// One of every outcome that wrote or refused a file and quotes its path,
/// each under `/srv/x/`.
fn file_statuses_quoting_a_path() -> Vec<Status> {
    vec![
        Status::SetupSaved(PathBuf::from("/srv/x/config/settings.yaml")),
        transferred(Purpose::Move, Landed::At("/srv/x/b/a.txt".into()), 1, 1),
        Status::CreatedDirs {
            count: 2,
            directory: "/srv/x/new".into(),
        },
        Status::AlreadyExists("/srv/x/taken".into()),
        Status::FileExists("/srv/x/a.tar".into()),
        Status::Linked {
            link: "/srv/x/l".into(),
            target: "/srv/x/t".into(),
        },
        Status::Extracted {
            count: 2,
            destination: "/srv/x/out".into(),
        },
        Status::AddedToArchive {
            count: 2,
            archive: "/srv/x/a.tar".into(),
            kept: 1,
        },
    ]
}

/// One of every partly failed batch, each failing first under `/srv/x/`.
fn failure_statuses_quoting_a_path() -> Vec<Status> {
    let first_error = || FailedEntry::new("/srv/x/a.txt", &"Permission denied");
    vec![
        Status::CopiedWithErrors {
            count: 2,
            error_count: 1,
            first_error: first_error(),
        },
        Status::MovedWithErrors {
            count: 2,
            error_count: 1,
            first_error: first_error(),
        },
        Status::DeletedWithErrors {
            count: 2,
            error_count: 1,
            first_error: first_error(),
        },
        Status::WipedWithErrors {
            count: 2,
            error_count: 1,
            first_error: first_error(),
        },
        Status::AttributesSetWithErrors {
            count: 2,
            error_count: 1,
            first_error: first_error(),
        },
    ]
}

/// Every message that quotes a path reads the same whole as in parts: the
/// info row cuts the parts, and must not be showing a different sentence.
#[test]
fn a_path_message_is_the_display_text_in_three_parts() {
    let statuses = view_edit_and_run_statuses_quoting_a_path()
        .into_iter()
        .chain(file_statuses_quoting_a_path())
        .chain(failure_statuses_quoting_a_path());
    for status in statuses {
        let message = status.path_message().expect("quotes a path");
        assert_eq!(message.to_string(), status.to_string());
        assert!(message.path.starts_with("/srv/x/"), "{message:?}");
    }
}

#[test]
fn an_edit_error_quotes_the_path_between_the_verb_and_the_reason() {
    let status = Status::CannotEdit {
        path: "/srv/x/u.txt".into(),
        error: "denied".to_owned(),
    };
    let edit = status.path_message().expect("quotes a path");
    assert_eq!(
        (
            edit.before.as_ref(),
            edit.path.as_ref(),
            edit.after.as_ref()
        ),
        ("Cannot edit ", "/srv/x/u.txt", ": denied")
    );
}

#[test]
fn a_message_without_a_path_has_no_path_parts() {
    assert_eq!(Status::UnmarkedAll.path_message(), None);
    let plain = PathMessage::from("Copied".to_owned());
    assert_eq!(plain.to_string(), "Copied");
    assert!(plain.path.is_empty() && plain.after.is_empty());
}

/// A program that had the terminal ends its part of the Ctrl+O transcript
/// with the same words the status bar shows: output when it ran, an error
/// line otherwise.
#[test]
fn a_run_outcome_reaches_the_status_and_closes_the_transcript() {
    use crate::console::ConsoleLine;
    use crate::models::Panel;
    let empty = || Panel::from_entries(PathBuf::from("/srv/x"), Vec::new());
    let mut app = App::with_panels(empty(), empty());
    let ran = Status::ProgramExited {
        program: PathBuf::from("/srv/x/run.sh"),
        status: std::process::ExitStatus::default(),
    };
    let ran_text = ran.to_string();

    app.record_run_outcome(Status::CommandOutput("Updated".into()));
    app.record_run_outcome(ran);
    app.record_run_outcome(Status::PackageUpdateCancelled);

    assert_eq!(app.status, Status::PackageUpdateCancelled);
    assert_eq!(
        app.console.visible(usize::MAX),
        [
            &ConsoleLine::Output("Updated".to_owned()),
            &ConsoleLine::Output(ran_text),
            &ConsoleLine::Error("Package update cancelled".to_owned()),
        ]
    );
}
