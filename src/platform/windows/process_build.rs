//! Purpose: Safe sample construction, SID cache, pre-flight control checks and
//! Win32 error mapping shared by `processes` and `process_control`.

use std::collections::HashMap;
use std::io;
use std::time::Duration;

use windows_sys::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_INVALID_PARAMETER};

use crate::models::{
    ControlError, ControlRefusal, MemoryUsage, Owner, Pid, Priority, PriorityChange, ProcessKey,
    ProcessKind, ProcessSample, ProcessState, Signal, Start, StartTime,
};
use crate::platform::windows_parse::{settable_from_nice, wide_buf_to_string, win32_from_settable};

/// Reused SID-to-username cache across samples in one `TaskList` session.
#[derive(Default)]
pub struct WinOwnerCache {
    pub(super) by_sid: HashMap<Vec<u8>, String>,
}

pub(super) struct HandleDetails {
    pub(super) start: Start,
    pub(super) cpu_time: Duration,
    pub(super) running_for: Option<Duration>,
    pub(super) image_path: Option<String>,
    pub(super) memory: Option<MemoryUsage>,
    pub(super) priority: Priority,
    pub(super) owner: Owner,
}

impl Default for HandleDetails {
    fn default() -> Self {
        Self {
            start: Start::Unknown,
            cpu_time: Duration::ZERO,
            running_for: None,
            image_path: None,
            memory: None,
            priority: Priority::Unknown,
            owner: Owner::Unknown,
        }
    }
}

pub(super) fn build_sample(
    raw_pid: u32,
    raw_parent: u32,
    threads: u32,
    exe_buf: &[u16],
    details: HandleDetails,
) -> Option<ProcessSample> {
    let pid = Pid::new(raw_pid)?;
    let name = wide_buf_to_string(exe_buf);
    let kind = if pid.get() == 4 {
        ProcessKind::KernelThread
    } else {
        ProcessKind::User
    };
    Some(ProcessSample {
        key: ProcessKey::new(pid, details.start),
        parent: Pid::new(raw_parent),
        command: details.image_path.unwrap_or_else(|| name.clone()),
        name,
        owner: details.owner,
        state: ProcessState::Running,
        priority: details.priority,
        threads: threads.max(1),
        cpu_time: details.cpu_time,
        memory: details.memory,
        running_for: details.running_for,
        kind,
    })
}

pub(super) fn validate_terminate(
    key: ProcessKey,
    signal: Signal,
    own: Pid,
) -> Result<(Pid, StartTime), ControlError> {
    if !matches!(signal, Signal::Terminate | Signal::Kill) {
        return Err(ControlError::Unsupported);
    }
    if key.pid() == own {
        return Err(ControlError::Refused(ControlRefusal::OwnProcess));
    }
    if key.pid().get() == 4 {
        return Err(ControlError::Refused(ControlRefusal::KernelThread));
    }
    let Start::Known(expected) = key.start() else {
        return Err(ControlError::Unverifiable);
    };
    Ok((key.pid(), expected))
}

pub(super) fn validate_priority(
    key: ProcessKey,
    change: PriorityChange,
) -> Result<(Pid, StartTime, u32), ControlError> {
    if key.pid().get() == 4 {
        return Err(ControlError::Refused(ControlRefusal::KernelThread));
    }
    let Start::Known(expected) = key.start() else {
        return Err(ControlError::Unverifiable);
    };
    let settable = match change {
        PriorityChange::Class(c) => c,
        PriorityChange::Nice(nice) => settable_from_nice(nice),
    };
    Ok((key.pid(), expected, win32_from_settable(settable)))
}

pub(super) fn map_win32_error(err: &io::Error) -> ControlError {
    match err.raw_os_error().and_then(|c| u32::try_from(c).ok()) {
        Some(ERROR_INVALID_PARAMETER) => ControlError::Gone,
        Some(ERROR_ACCESS_DENIED) => ControlError::PermissionDenied,
        _ if err.kind() == io::ErrorKind::PermissionDenied => ControlError::PermissionDenied,
        _ => ControlError::Failed(err.kind()),
    }
}
