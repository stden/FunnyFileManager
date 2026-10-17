//! Purpose: Tests for the sentences the task list shows after an action: a
//! nice step's notice, and the Error dialog's line for every way a signal
//! or a renice can fail, worded by what was asked.

use super::tests_support::{listing, process};
use super::{
    Attempt, ControlError, ControlRefusal, Failure, NiceStep, Notice, Operation, Owner,
    ProcessSample, Severity, Target,
};

fn firefox(owner: Owner) -> Target {
    let sample = ProcessSample {
        owner,
        ..process(4242, "firefox")
    };
    let frozen = listing(vec![sample]).freeze_targets(Operation::Signal);
    frozen.targets()[0].clone()
}

fn line(error: ControlError, attempt: Attempt) -> String {
    Failure::new(firefox(Owner::Named("root".into())), attempt, error).to_string()
}

#[test]
fn a_nice_step_says_which_way_and_for_how_many() {
    let raised = Notice::Stepped {
        step: NiceStep::Raise,
        count: 2,
    };
    assert_eq!(
        raised.to_string(),
        "Priority raised one step for 2 processes."
    );
    let lowered = Notice::Stepped {
        step: NiceStep::Lower,
        count: 1,
    };
    assert_eq!(
        lowered.to_string(),
        "Priority lowered one step for 1 process."
    );
    assert_eq!(lowered.severity(), Severity::Information);
}

/// Every error's line for a signal, as spec section 9 words it.
const SIGNAL_LINES: &[(ControlError, &str)] = &[
    (
        ControlError::Gone,
        "4242 firefox has already exited; nothing was sent.",
    ),
    (
        ControlError::Replaced,
        "PID 4242 now belongs to another process; nothing was sent.",
    ),
    (
        ControlError::PermissionDenied,
        "4242 firefox belongs to another user (root).",
    ),
    (
        ControlError::Critical,
        "4242 is a critical system process; ending it stops Windows.",
    ),
    (
        ControlError::Unverifiable,
        "FFM cannot confirm which process 4242 is; nothing was sent.",
    ),
    (
        ControlError::ReadOnly,
        "This is a replayed session; processes are not signalled.",
    ),
    (
        ControlError::Refused(ControlRefusal::Init),
        "PID 1 (init) is never signalled from here.",
    ),
    (ControlError::Unsupported, "Not supported on this platform."),
    (
        ControlError::Failed(std::io::ErrorKind::TimedOut),
        "Could not signal 4242 firefox: timed out.",
    ),
];

#[test]
fn a_failed_signal_reads_as_the_specs_sentence() {
    for &(error, sentence) in SIGNAL_LINES {
        assert_eq!(line(error, Attempt::Signal), sentence, "{error:?}");
    }
}

#[test]
fn a_failed_renice_says_nothing_was_changed() {
    let lower = Attempt::Renice(NiceStep::Lower);
    let cases = [
        (
            ControlError::Gone,
            "4242 firefox has already exited; nothing was changed.",
        ),
        (
            ControlError::Replaced,
            "PID 4242 now belongs to another process; nothing was changed.",
        ),
        (
            ControlError::Unverifiable,
            "FFM cannot confirm which process 4242 is; nothing was changed.",
        ),
        (
            ControlError::ReadOnly,
            "This is a replayed session; priorities are not changed.",
        ),
        (
            ControlError::Failed(std::io::ErrorKind::InvalidInput),
            "Could not renice 4242 firefox: invalid input parameter.",
        ),
    ];
    for (error, sentence) in cases {
        assert_eq!(line(error, lower), sentence, "{error:?}");
    }
}

/// Spec S11: raising a priority takes `CAP_SYS_NICE` even on one's own
/// process; lowering it, like a signal, is refused only as another user's.
#[test]
fn a_permission_refusal_is_worded_by_which_way_the_priority_moved() {
    assert_eq!(
        line(
            ControlError::PermissionDenied,
            Attempt::Renice(NiceStep::Raise)
        ),
        "Only root can raise a process's priority (CAP_SYS_NICE)."
    );
    assert_eq!(
        line(
            ControlError::PermissionDenied,
            Attempt::Renice(NiceStep::Lower)
        ),
        "4242 firefox belongs to another user (root)."
    );
}

#[test]
fn another_users_process_is_named_by_uid_or_not_at_all() {
    let denied = |owner| {
        Failure::new(
            firefox(owner),
            Attempt::Signal,
            ControlError::PermissionDenied,
        )
        .to_string()
    };
    assert_eq!(
        denied(Owner::Uid(1001)),
        "4242 firefox belongs to another user (uid 1001)."
    );
    assert_eq!(
        denied(Owner::Unknown),
        "4242 firefox belongs to another user."
    );
}

#[test]
fn a_frozen_target_keeps_its_owner() {
    let target = firefox(Owner::Named("alice".into()));
    assert_eq!(target.owner(), &Owner::Named("alice".into()));
}

#[test]
fn the_new_refusals_read_as_sentences() {
    assert_eq!(
        ControlRefusal::ClassAtLimit.to_string(),
        "The priority class is already at its limit."
    );
    assert_eq!(
        ControlRefusal::UnknownPriority.to_string(),
        "The priority of this process is not known."
    );
}
