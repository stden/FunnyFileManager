//! Purpose: The Linux process reader, over a `/proc` tree: in production the
//! real one, in tests a directory of fixture files.
//!
//! Each file is read with `std::fs` only, up to a cap, into one buffer every
//! read of a sample reuses, and decoded lossily.

mod commands;
mod control;
mod files;
mod net;
mod parse;
mod processes;
mod system_counters;

#[cfg(target_os = "linux")]
pub(super) use control::ProcessControl;

use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::Instant;

use crate::directory::owner::Owners;
use crate::models::{ProcessReader, Sample, SampleError, SystemPart};

use commands::CommandCache;
use files::Buffer;
use parse::{ClockTicks, PageSize};

/// Where the kernel mounts the process table.
#[cfg(target_os = "linux")]
pub(super) const PROC: &str = "/proc";
#[cfg(target_os = "linux")]
const SYS_NET: &str = "/sys/class/net";
#[cfg(target_os = "linux")]
const RESOLV_CONF: &str = "/etc/resolv.conf";

/// The reader of one task-list session. It keeps, for the session, the user
/// names already looked up and the command lines already read.
pub(super) struct ProcFs {
    root: PathBuf,
    sys_net: PathBuf,
    resolv_conf: PathBuf,
    ticks: ClockTicks,
    page: PageSize,
    users: Owners,
    commands: CommandCache,
    buffer: Buffer,
    started: Instant,
}

impl ProcFs {
    /// A reader of the process table mounted at `root`, converting CPU times
    /// at `ticks` and memory at `page`.
    fn new(root: PathBuf, ticks: ClockTicks, page: PageSize) -> Self {
        let sys_net = root.join("sys/class/net");
        let resolv_conf = root.join("etc/resolv.conf");
        Self {
            root,
            sys_net,
            resolv_conf,
            ticks,
            page,
            users: Owners::default(),
            commands: CommandCache::default(),
            buffer: Buffer::default(),
            started: Instant::now(),
        }
    }

    /// The reader of this system's `/proc`, with the clock rate and page size
    /// the kernel reports; either being zero fails every sample to come.
    #[cfg(target_os = "linux")]
    pub(super) fn system() -> Result<Self, SampleError> {
        let ticks = system_ticks().ok_or(SampleError::Malformed(SystemPart::ClockRate))?;
        let page = PageSize::new(crate::platform::page_size())
            .ok_or(SampleError::Malformed(SystemPart::PageSize))?;
        let mut reader = Self::new(PathBuf::from(PROC), ticks, page);
        reader.sys_net = PathBuf::from(SYS_NET);
        reader.resolv_conf = PathBuf::from(RESOLV_CONF);
        Ok(reader)
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
        let network = net::read_linux_network(
            &self.root,
            &self.sys_net,
            &self.resolv_conf,
            self.started.elapsed(),
            &mut self.buffer,
        );
        Ok(Sample {
            system,
            processes,
            unreadable,
            network,
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
mod tests_net;
#[cfg(test)]
mod tests_net_parse;
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
