//! Purpose: Interaction records preserve timings and redact outcome payloads.

use super::*;

#[test]
fn completion_records_a_correlatable_action_and_microsecond_latency() {
    assert_eq!(
        completion(
            7,
            "Navigation(NavDown)",
            Duration::from_micros(125),
            "handled"
        ),
        "COMPLETE id=7 action=Navigation(NavDown) outcome=handled processing_us=125"
    );
}

#[test]
fn status_records_never_include_command_output_or_error_payloads() {
    use crate::status::Status;
    assert_eq!(
        status_kind(&Status::Error("private password".to_owned())),
        "Error"
    );
    assert_eq!(
        status_kind(&Status::CommandOutput("secret".to_owned())),
        "CommandOutput"
    );
    assert_eq!(status_kind(&Status::Cleared), "Cleared");
}

#[test]
fn interaction_logs_io_errors_and_process_status_kinds() {
    use crate::models::{App, Panel};
    use crate::status::Status;
    use std::{io, path::PathBuf, process::ExitStatus};

    let panel = || Panel::from_entries(PathBuf::from("."), Vec::new());
    let mut app = App::with_panels(panel(), panel());
    app.set_status(Status::CannotRun {
        program: PathBuf::from("tool"),
        error: "denied".to_owned(),
    });
    let ix = Interaction::start("Open".into(), &app);
    let failure: io::Result<()> = Err(io::Error::from(io::ErrorKind::PermissionDenied));
    ix.finish(&app, &failure);

    app.set_status(Status::ProgramExited {
        program: PathBuf::from("tool"),
        status: ExitStatus::default(),
    });
    let ix2 = Interaction::start("Run".into(), &app);
    ix2.finish(&app, &Ok(()));
}

#[test]
fn a_failed_interaction_logs_the_error_kind_without_its_payload() {
    use crate::models::Panel;
    use std::{cell::RefCell, io, path::PathBuf};

    let panel = || Panel::from_entries(PathBuf::from("."), Vec::new());
    let app = App::with_panels(panel(), panel());
    let actions = RefCell::new(Vec::new());
    let errors = RefCell::new(Vec::new());
    let interaction = Interaction::start("Editor/Save".into(), &app);
    let result: io::Result<()> = Err(io::Error::new(
        io::ErrorKind::PermissionDenied,
        "private password",
    ));
    interaction.finish_with(
        &app,
        &result,
        |line| actions.borrow_mut().push(line.to_owned()),
        |line| errors.borrow_mut().push(line.to_owned()),
    );
    let actions = actions.into_inner().join("\n");
    let errors = errors.into_inner().join("\n");
    assert!(actions.contains("outcome=error"));
    assert!(errors.contains("PermissionDenied"));
    assert!(!actions.contains("private password"));
    assert!(!errors.contains("private password"));
}
