//! Purpose: Why a sample could not be taken and why a signal or a priority
//! change was not applied — the variants callers branch on.
//!
//! Each reads as one fixed English sentence that names no path and quotes no
//! command line; the dialogs that name a target add its PID and name around
//! it.

use std::fmt;
use std::io;

use super::ControlRefusal;

/// Why no sample could be taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleError {
    /// This platform has no process reader.
    Unsupported,
    Unreadable {
        part: SystemPart,
        kind: io::ErrorKind,
    },
    Malformed(SystemPart),
    /// The sampling thread could not be started.
    WorkerUnavailable,
    /// The sampling thread ended.
    WorkerStopped,
    /// The sampling thread did not answer within the stall limit and was
    /// abandoned before the first table arrived.
    WorkerStalled,
}

impl fmt::Display for SampleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => {
                f.write_str("The process table is not available on this platform.")
            }
            Self::Unreadable { part, kind } => {
                write!(f, "The process table cannot be read ({part}: {kind}).")
            }
            Self::Malformed(part) => {
                write!(f, "The process table cannot be read ({part}: malformed).")
            }
            Self::WorkerUnavailable => f.write_str("The process reader could not start."),
            Self::WorkerStopped => f.write_str("The process reader stopped."),
            Self::WorkerStalled => f.write_str("The process reader is not responding."),
        }
    }
}

impl std::error::Error for SampleError {}

/// The part of the system a whole-sample error comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemPart {
    ProcessList,
    CpuTimes,
    ClockRate,
    PageSize,
}

impl fmt::Display for SystemPart {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ProcessList => "process list",
            Self::CpuTimes => "CPU times",
            Self::ClockRate => "clock rate",
            Self::PageSize => "page size",
        })
    }
}

/// Why a signal or a priority change was not applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlError {
    /// The process exited before anything was sent.
    Gone,
    /// The PID now names a different process.
    Replaced,
    /// The kernel refused: the process belongs to another user (`EPERM`), or
    /// lowering nice needs `CAP_SYS_NICE` (`EACCES`), even on the user's own
    /// process. The sentence names neither cause; the dialog words it from
    /// the operation (spec S11).
    PermissionDenied,
    /// Ending it would stop the system (Windows critical processes).
    Critical,
    /// The source cannot tell this process from a successor.
    Unverifiable,
    /// A replayed session never controls processes.
    ReadOnly,
    /// The platform itself refused, whatever the caller checked first: no
    /// signal goes to FFM itself, init or a kernel thread.
    Refused(ControlRefusal),
    Unsupported,
    Failed(io::ErrorKind),
}

impl fmt::Display for ControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gone => f.write_str("The process has already exited; nothing was sent."),
            Self::Replaced => {
                f.write_str("The PID now belongs to another process; nothing was sent.")
            }
            Self::PermissionDenied => f.write_str("Permission denied."),
            Self::Critical => {
                f.write_str("It is a critical system process; ending it stops Windows.")
            }
            Self::Unverifiable => {
                f.write_str("FFM cannot confirm which process it is; nothing was sent.")
            }
            Self::ReadOnly => {
                f.write_str("This is a replayed session; processes are not signalled.")
            }
            Self::Refused(refusal) => refusal.fmt(f),
            Self::Unsupported => f.write_str("Not supported on this platform."),
            Self::Failed(kind) => write!(f, "The system refused the request: {kind}."),
        }
    }
}

impl std::error::Error for ControlError {}
