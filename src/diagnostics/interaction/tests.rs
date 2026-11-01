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
