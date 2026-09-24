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

#[cfg(target_os = "linux")]
pub(super) use control::ProcessControl;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::directory::owner::Owners;
use crate::models::{
    Owner, Pid, ProcessKey, ProcessReader, ProcessSample, ProcessState, Sample, SampleError, Start,
    SystemPart, SystemSample,
};

use commands::CommandCache;
use files::{vanished, Buffer, ProcessFile, SystemFile, SYSTEM_CAP};
use parse::{ClockTicks, PageSize, Stat};

/// Where the kernel mounts the process table.
#[cfg(target_os = "linux")]
pub(super) const PROC: &str = "/proc";

/// Why a process is left out of a sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Skip {
    /// It exited during the scan: left out without a word.
    Vanished,
    /// Its `stat` could not be read or parsed: counted as unreadable.
    Unreadable,
}

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

impl ProcFs {
    /// The system-wide counters. Without `/proc/stat` there are no CPU
    /// shares, and the sample fails; the other files only lose their meter.
    fn system_sample(&mut self) -> Result<SystemSample, SampleError> {
        let ticks = self.ticks;
        let times = self
            .read_system(SystemFile::Stat, |text| parse::cpu_times(text, ticks))
            .map_err(|error| SampleError::Unreadable {
                part: SystemPart::CpuTimes,
                kind: error.kind(),
            })?
            .ok_or(SampleError::Malformed(SystemPart::CpuTimes))?;
        let memory = self
            .read_system(SystemFile::MemInfo, parse::memory_info)
            .ok();
        Ok(SystemSample {
            total: times.total,
            cores: times.cores,
            memory: memory.and_then(|info| info.memory),
            swap: memory.and_then(|info| info.swap),
            load: self.optional(SystemFile::LoadAverage, parse::load_average),
            uptime: self.optional(SystemFile::Uptime, parse::uptime),
            running: times.running,
        })
    }

    /// A system-wide file, parsed.
    fn read_system<T>(&mut self, file: SystemFile, parse: impl FnOnce(&str) -> T) -> io::Result<T> {
        let path = self.root.join(file.name());
        self.buffer.read(&path, SYSTEM_CAP).map(|text| parse(&text))
    }

    /// An optional system-wide file's value: `None` when it cannot be read
    /// or says nothing.
    fn optional<T>(
        &mut self,
        file: SystemFile,
        parse: impl FnOnce(&str) -> Option<T>,
    ) -> Option<T> {
        self.read_system(file, parse).ok().flatten()
    }

    /// Every process `entries` lists, each once, and how many could not be
    /// read. A listing that fails part way fails the sample: a table missing
    /// the rest of it would look complete.
    fn processes(
        &mut self,
        entries: fs::ReadDir,
        uptime: Option<Duration>,
    ) -> Result<(Vec<ProcessSample>, u32), SampleError> {
        self.users.forget_failed_lookups();
        self.commands.start_sample();
        let (mut processes, mut unreadable) = (Vec::new(), 0_u32);
        for entry in entries {
            let entry = entry.map_err(|error| unlisted(&error))?;
            let Some(pid) = entry.file_name().to_str().and_then(parse::directory_pid) else {
                continue;
            };
            match self.process(&entry.path(), pid, uptime) {
                Ok(process) => processes.push(process),
                Err(Skip::Unreadable) => unreadable = unreadable.saturating_add(1),
                Err(Skip::Vanished) => {}
            }
        }
        processes.sort_by_key(|process| process.key);
        processes.dedup_by_key(|process| process.key);
        Ok((processes, unreadable))
    }

    /// The process `pid`, whose files are in `dir`.
    fn process(
        &mut self,
        dir: &Path,
        pid: Pid,
        uptime: Option<Duration>,
    ) -> Result<ProcessSample, Skip> {
        let (ticks, page) = (self.ticks, self.page);
        let stat = self
            .read(dir, ProcessFile::Stat, |text| parse::stat(text, pid, ticks))?
            .flatten()
            .ok_or(Skip::Unreadable)?;
        let memory = self
            .read(dir, ProcessFile::Statm, |text| parse::statm(text, page))?
            .flatten();
        let uid = self
            .read(dir, ProcessFile::Status, parse::effective_uid)?
            .flatten();
        let key = ProcessKey::new(pid, Start::Known(stat.start));
        let command = self.command(dir, key, &stat)?;
        Ok(ProcessSample {
            key,
            parent: stat.parent,
            name: stat.name,
            command,
            owner: uid.map_or(Owner::Unknown, |uid| owner(&mut self.users, uid)),
            state: stat.state,
            priority: stat.priority,
            threads: stat.threads,
            cpu_time: stat.cpu_time,
            memory,
            running_for: uptime.map(|uptime| uptime.saturating_sub(stat.started)),
            kind: stat.kind,
        })
    }

    /// The command line of the process `key`: kept from an earlier sample
    /// while its name is unchanged, never read while it sleeps in the kernel
    /// uncached (the read would wait on its mmap lock), and `[name]` when it
    /// has none or it cannot be read.
    fn command(&mut self, dir: &Path, key: ProcessKey, stat: &Stat) -> Result<String, Skip> {
        if let Some(line) = self.commands.reuse(key, &stat.name) {
            return Ok(line);
        }
        let unread = || parse::command_line("", &stat.name);
        if stat.state == ProcessState::DiskSleep {
            return Ok(unread());
        }
        let read = self.read(dir, ProcessFile::Cmdline, |text| {
            parse::command_line(text, &stat.name)
        })?;
        let Some(line) = read else {
            return Ok(unread());
        };
        self.commands.keep(key, &stat.name, &line);
        Ok(line)
    }

    /// `file` of the process whose files are in `dir`, parsed: `Ok(None)`
    /// when it cannot be read, and [`Skip::Vanished`] when that is because
    /// the process has exited.
    fn read<T>(
        &mut self,
        dir: &Path,
        file: ProcessFile,
        parse: impl FnOnce(&str) -> T,
    ) -> Result<Option<T>, Skip> {
        match self.buffer.read(&dir.join(file.name()), file.cap()) {
            Ok(text) => Ok(Some(parse(&text))),
            Err(error) if vanished(&error) => Err(Skip::Vanished),
            Err(_) => Ok(None),
        }
    }
}

/// The process table could not be listed.
fn unlisted(error: &io::Error) -> SampleError {
    SampleError::Unreadable {
        part: SystemPart::ProcessList,
        kind: error.kind(),
    }
}

/// The user `uid` names: by name, or by number when it has no passwd entry
/// or its lookup failed (asked again next sample).
fn owner(users: &mut Owners, uid: u32) -> Owner {
    users.name_of_uid(uid).map_or(Owner::Uid(uid), Owner::Named)
}

#[cfg(test)]
mod tests_control;
#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests_control_kernel;
#[cfg(test)]
mod tests_parse;
#[cfg(test)]
mod tests_parse_system;
#[cfg(test)]
mod tests_reader;
#[cfg(test)]
mod tests_reader_cache;
#[cfg(test)]
mod tests_reader_system;
#[cfg(test)]
mod tests_robustness;
