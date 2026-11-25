use super::*;
use crate::models::App;

mod expiry;
mod legacy_commands;
mod legacy_editor_and_platform;
mod legacy_file_ops;
mod path_message;

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

fn assert_status_text(cases: &[(Status, &str)]) {
    for (status, expected) in cases {
        assert_eq!(&status.to_string(), expected, "mismatch for {status:?}");
    }
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
