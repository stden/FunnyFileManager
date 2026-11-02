//! Purpose: Tests for the sample and control errors: one fixed sentence each.

use std::io::ErrorKind;

use super::{ControlError, ControlRefusal, SampleError, SystemPart};

const SAMPLE_ERROR_SENTENCES: [(SampleError, &str); 8] = [
    (
        SampleError::Unsupported,
        "The process table is not available on this platform.",
    ),
    (
        SampleError::Unreadable {
            part: SystemPart::ProcessList,
            kind: ErrorKind::PermissionDenied,
        },
        "The process table cannot be read (process list: permission denied).",
    ),
    (
        SampleError::Malformed(SystemPart::CpuTimes),
        "The process table cannot be read (CPU times: malformed).",
    ),
    (
        SampleError::Malformed(SystemPart::ClockRate),
        "The process table cannot be read (clock rate: malformed).",
    ),
    (
        SampleError::Malformed(SystemPart::PageSize),
        "The process table cannot be read (page size: malformed).",
    ),
    (
        SampleError::WorkerUnavailable,
        "The process reader could not start.",
    ),
    (SampleError::WorkerStopped, "The process reader stopped."),
    (
        SampleError::WorkerStalled,
        "The process reader is not responding.",
    ),
];

#[test]
fn each_sample_error_reads_as_one_sentence() {
    for (error, sentence) in SAMPLE_ERROR_SENTENCES {
        assert_eq!(error.to_string(), sentence);
    }
}

const CONTROL_ERROR_SENTENCES: [(ControlError, &str); 9] = [
    (
        ControlError::Gone,
        "The process has already exited; nothing was sent.",
    ),
    (
        ControlError::Replaced,
        "The PID now belongs to another process; nothing was sent.",
    ),
    // Neutral: the refusal may be a nice-down on the user's own process.
    (ControlError::PermissionDenied, "Permission denied."),
    (
        ControlError::Critical,
        "It is a critical system process; ending it stops Windows.",
    ),
    (
        ControlError::Unverifiable,
        "FFM cannot confirm which process it is; nothing was sent.",
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
        ControlError::Failed(ErrorKind::InvalidInput),
        "The system refused the request: invalid input parameter.",
    ),
];

#[test]
fn each_control_error_reads_as_one_sentence() {
    for (error, sentence) in CONTROL_ERROR_SENTENCES {
        assert_eq!(error.to_string(), sentence);
    }
}
