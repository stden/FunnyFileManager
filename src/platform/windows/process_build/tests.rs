//! Purpose: Unit tests for Windows sample building, pre-flight validation, and Win32 error mapping.

use std::io;

use super::{build_sample, map_win32_error, validate_priority, validate_terminate, HandleDetails};
use crate::models::{
    ControlError, ControlRefusal, Niceness, Pid, PriorityChange, ProcessKey, ProcessKind, Signal,
    Start, StartTime,
};

#[test]
fn build_sample_marks_pid_four_as_kernel_thread_and_rejects_pid_zero() {
    assert!(build_sample(0, 0, 1, &[0], HandleDetails::default()).is_none());
    let sample = build_sample(4, 0, 0, &[u16::from(b'S'), 0], HandleDetails::default()).unwrap();
    assert_eq!(sample.kind, ProcessKind::KernelThread);
    assert_eq!(sample.threads, 1);
}

#[test]
fn validate_terminate_and_priority_cover_unsupported_kernel_unverifiable_and_nice() {
    let own = Pid::new(100).unwrap();
    let kernel = ProcessKey::new(Pid::new(4).unwrap(), Start::Known(StartTime::new(1)));
    let unknown = ProcessKey::new(Pid::new(200).unwrap(), Start::Unknown);
    let known = ProcessKey::new(Pid::new(200).unwrap(), Start::Known(StartTime::new(9)));

    assert_eq!(
        validate_terminate(known, Signal::Hangup, own),
        Err(ControlError::Unsupported)
    );
    assert_eq!(
        validate_terminate(kernel, Signal::Kill, own),
        Err(ControlError::Refused(ControlRefusal::KernelThread))
    );
    assert_eq!(
        validate_terminate(unknown, Signal::Kill, own),
        Err(ControlError::Unverifiable)
    );
    let nice = PriorityChange::Nice(Niceness::try_from(5).unwrap());
    assert_eq!(
        validate_priority(kernel, nice),
        Err(ControlError::Refused(ControlRefusal::KernelThread))
    );
    assert!(validate_priority(known, nice).is_ok());
}

#[test]
fn map_win32_error_maps_invalid_param_access_denied_and_other_errors() {
    assert_eq!(
        map_win32_error(&io::Error::from_raw_os_error(87)),
        ControlError::Gone
    );
    assert_eq!(
        map_win32_error(&io::Error::from_raw_os_error(5)),
        ControlError::PermissionDenied
    );
    let kind_denied = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
    assert_eq!(
        map_win32_error(&kind_denied),
        ControlError::PermissionDenied
    );
    assert_eq!(
        map_win32_error(&io::Error::other("other")),
        ControlError::Failed(io::ErrorKind::Other)
    );
}
