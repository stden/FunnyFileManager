//! Purpose: unit tests for every notice sentence, its severity, and a failure's
//! target and error.

use super::*;

#[test]
fn a_sent_signal_and_a_renice_read_as_information() {
    let sent = Notice::Sent {
        signal: Signal::Terminate,
        count: 2,
    };
    assert_eq!(sent.to_string(), "Sent SIGTERM to 2 processes.");
    let reniced = Notice::Reniced {
        count: 1,
        nice: nice(5),
    };
    assert_eq!(reniced.to_string(), "Nice set to 5 for 1 process.");
    for notice in [sent, reniced] {
        assert_eq!(notice.severity(), Severity::Information);
    }
}

#[test]
fn marks_are_information_and_a_refusal_is_a_warning() {
    let marked = Notice::Marked {
        changed: 3,
        total: 5,
    };
    assert_eq!(
        marked.to_string(),
        "Marks changed on 3 processes; 5 marked."
    );
    assert_eq!(marked.severity(), Severity::Information);
    let refused = Notice::Refused(ControlRefusal::KernelThread);
    assert_eq!(refused.to_string(), "Kernel threads ignore signals.");
    assert_eq!(refused.severity(), Severity::Warning);
}

#[test]
fn a_failure_keeps_its_target_and_error() {
    let target = vim().targets()[0].clone();
    let failure = Failure::new(target.clone(), Attempt::Signal, ControlError::Gone);
    assert_eq!(failure.target(), &target);
    assert_eq!(failure.attempt(), Attempt::Signal);
    assert_eq!(failure.error(), ControlError::Gone);
}
