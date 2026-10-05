//! Purpose: The Linux process reader, over a `/proc` tree: in production the
//! real one, in tests a directory of fixture files.
//!
//! The text of every file is parsed by pure functions in `parse`, which never
//! index, unwrap or overflow: `/proc` is written by the kernel, but a process
//! chooses its own name and command line, and a file can end at any byte when
//! its process exits mid-read.
//!
//! Each file is read with `std::fs` only, up to a cap, into one buffer every
//! read of a sample reuses, and decoded lossily. A process that exits during
//! the scan is left out without a word; one whose `stat` cannot be read or
//! parsed is left out and counted; an optional file that cannot be read
//! costs only its own field.

mod commands;
mod control;
mod files;
mod parse;
mod processes;
mod system_counters;

#[cfg(target_os = "linux")]
pub(super) use control::ProcessControl;

use std::fs;
use std::io;
use std::path::PathBuf;

use crate::directory::owner::Owners;
use crate::models::{ProcessReader, Sample, SampleError, SystemPart};

use commands::CommandCache;
use files::Buffer;
use parse::{ClockTicks, PageSize};

/// Where the kernel mounts the process table.
#[cfg(target_os = "linux")]
pub(super) const PROC: &str = "/proc";

/// The reader of one task-list session. It keeps, for the session, the user
/// names already looked up and the command lines already read.
pub(super) struct ProcFs {
    root: PathBuf,
    ticks: ClockTicks,
    page: PageSize,
    users: Owners,
    commands: CommandCache,
    buffer: Buffer,
}

impl ProcFs {
    /// A reader of the process table mounted at `root`, converting CPU times
    /// at `ticks` and memory at `page`.
    fn new(root: PathBuf, ticks: ClockTicks, page: PageSize) -> Self {
        Self {
            root,
            ticks,
            page,
            users: Owners::default(),
            commands: CommandCache::default(),
            buffer: Buffer::default(),
        }
    }

    /// The reader of this system's `/proc`, with the clock rate and page size
    /// the kernel reports; either being zero fails every sample to come.
    #[cfg(target_os = "linux")]
    pub(super) fn system() -> Result<Self, SampleError> {
        let ticks = system_ticks().ok_or(SampleError::Malformed(SystemPart::ClockRate))?;
        let page = PageSize::new(crate::platform::page_size())
            .ok_or(SampleError::Malformed(SystemPart::PageSize))?;
        Ok(Self::new(PathBuf::from(PROC), ticks, page))
    }
}

/// The kernel's clock rate, or `None` should it report zero.
#[cfg(target_os = "linux")]
fn system_ticks() -> Option<ClockTicks> {
    ClockTicks::new(crate::platform::clock_ticks_per_second())
}

impl ProcessReader for ProcFs {
    fn sample(&mut self) -> Result<Sample, SampleError> {
        let entries = fs::read_dir(&self.root).map_err(|error| unlisted(&error))?;
        let system = self.system_sample()?;
        let (processes, unreadable) = self.processes(entries, system.uptime)?;
        Ok(Sample {
            system,
            processes,
            unreadable,
        })
    }
}

/// The process table could not be listed.
fn unlisted(error: &io::Error) -> SampleError {
    SampleError::Unreadable {
        part: SystemPart::ProcessList,
        kind: error.kind(),
    }
}

#[cfg(test)]
mod tests_control;
#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests_control_kernel;
#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests_control_refusals;
#[cfg(test)]
mod tests_parse;
#[cfg(test)]
mod tests_parse_meminfo;
#[cfg(test)]
mod tests_parse_stat;
#[cfg(test)]
mod tests_parse_system;
#[cfg(test)]
mod tests_reader;
#[cfg(test)]
mod tests_reader_cache;
#[cfg(test)]
mod tests_reader_fields;
#[cfg(test)]
mod tests_reader_listing;
#[cfg(test)]
mod tests_reader_system;
#[cfg(test)]
mod tests_robustness;
#[cfg(test)]
mod tests_robustness_truncation;
