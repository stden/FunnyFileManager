//! Purpose: A `claude` that is missing, vanishes, times out or fails, and what the report says.

use super::*;

#[test]
fn a_cli_that_left_path_is_missing() {
    let fixture = fixture();
    fs::remove_file(fixture.machine.bin().join("claude")).unwrap();
    assert_eq!(
        fixture.register().outcome,
        Outcome::Failed(Failure::CliMissing)
    );
}

#[test]
fn a_cli_that_vanishes_while_running_is_missing() {
    let fixture = fixture();
    fixture.machine.claude().behave(ClaudeBehaviour::Vanish);
    assert_eq!(
        fixture.register().outcome,
        Outcome::Failed(Failure::CliMissing)
    );
}

#[test]
fn a_cli_that_times_out_says_what_to_run_by_hand() {
    let fixture = fixture();
    fixture.machine.claude().behave(ClaudeBehaviour::TimeOut);
    let Outcome::Failed(failure) = fixture.register().outcome else {
        panic!("a timeout must fail");
    };
    assert!(matches!(failure, Failure::CliTimedOut { .. }));
    let text = failure.describe("x");
    assert!(
        text.starts_with("claude did not finish within 30 s. Run by hand: claude mcp add"),
        "{text}"
    );
}

#[test]
fn a_non_zero_exit_reports_the_last_stderr_line() {
    let fixture = fixture();
    fixture.machine.claude().behave(ClaudeBehaviour::Fail(
        "warming up\nError: not signed in".to_owned(),
    ));
    let registration = fixture.register();
    let Outcome::Failed(Failure::CliFailed { exit, message, .. }) = registration.outcome else {
        panic!("a failed exit must fail");
    };
    assert_eq!(exit, Some(1));
    assert_eq!(message, "Error: not signed in");
}

#[test]
fn a_silent_failure_falls_back_to_stdout_then_to_a_sentence() {
    let silent = Captured {
        exit: Exit::Failure(None),
        stdout: "\n".to_owned(),
        stderr: String::new(),
    };
    let chatty = Captured {
        stdout: "usage: claude\n".to_owned(),
        ..silent.clone()
    };
    let fixture = fixture();
    for (captured, expected) in [
        (silent, "claude failed without saying why"),
        (chatty, "usage: claude"),
    ] {
        let mut host = fixture.host.clone();
        host.runner = std::sync::Arc::new(Answer(captured));
        let Outcome::Failed(Failure::CliFailed { message, exit, .. }) =
            register(&host, &fixture.desired).outcome
        else {
            panic!("must fail");
        };
        assert_eq!(message, expected);
        assert_eq!(exit, None);
    }
}

#[test]
fn a_runner_error_is_a_cli_failure_with_its_message() {
    let fixture = fixture();
    let mut host = fixture.host.clone();
    host.runner = Host::default().runner;
    let Outcome::Failed(Failure::CliFailed { exit, message, .. }) =
        register(&host, &fixture.desired).outcome
    else {
        panic!("an unsupported runner must fail");
    };
    assert_eq!(exit, None);
    assert!(message.contains("no program runner"), "{message}");
}
