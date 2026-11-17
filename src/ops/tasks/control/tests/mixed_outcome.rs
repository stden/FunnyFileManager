//! Purpose: A delivery that reached some targets and failed for others says
//! both: what was sent, then why each of the rest was not.

use super::*;

/// The reported case: the user's own `sleep`, PID 1 and root's tmux server,
/// all marked, F8 Enter. init was skipped at the confirmation, the tmux
/// server refused, and the Error dialog named only the refusal, though the
/// `sleep` had been ended.
#[test]
fn a_mixed_delivery_reports_what_was_sent_and_what_failed() {
    let mut tasks = TaskApp::open(
        vec![
            named(1, "systemd"),
            named(1002, "sleep"),
            named(2000, "tmux"),
        ],
        Privilege::User,
    );
    tasks
        .host
        .fail(key_of(2000), ControlError::PermissionDenied);
    tasks.list().invert_marks();
    confirm_terminate(tasks.list());
    press(&mut tasks.app, ConfirmButton::Terminate);

    let Modal::Error { done, failures } = tasks.list().modal() else {
        panic!("the error dialog is open");
    };
    let sent = Notice::Sent {
        signal: Signal::Terminate,
        count: 1,
    };
    assert_eq!(*done, Some(sent));
    assert_eq!(
        done.map(|done| done.to_string()).as_deref(),
        Some("Sent SIGTERM to 1 process.")
    );
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].target().key(), key_of(2000));
}

/// When nothing was delivered there is nothing to report as done.
#[test]
fn a_delivery_that_failed_everywhere_reports_only_the_failures() {
    let mut tasks = firefox();
    tasks.host.fail(key_of(4242), ControlError::Gone);
    confirm_terminate(tasks.list());
    press(&mut tasks.app, ConfirmButton::Terminate);
    let Modal::Error { done, failures } = tasks.list().modal() else {
        panic!("the error dialog is open");
    };
    assert_eq!(*done, None);
    assert_eq!(failures.len(), 1);
}
