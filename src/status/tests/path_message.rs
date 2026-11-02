//! Purpose: `Status::path_message` — each message split around the path it quotes.

use super::*;

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
