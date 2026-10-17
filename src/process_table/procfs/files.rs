//! Purpose: The files the process reader reads from `/proc`, how much of
//! each, and the one buffer every read of a sample goes through.
//!
//! Each read takes at most its file's cap, so a hostile or runaway file can
//! cost the reader no more than that, and is decoded lossily: `/proc` text
//! is mostly the kernel's, but a name or a command line is the process's
//! own and need not be UTF-8.

use std::borrow::Cow;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// Linux's `ESRCH`, "no such process": what a file of a process that exited
/// mid-read may answer. Named with the Linux value, so the mapping compiles
/// and is tested on every CI system.
pub(super) const ESRCH: i32 = 3;

/// How much of a system-wide file is read: `procs_running` comes after the
/// `intr` line of `/proc/stat`, which runs long on a machine with many
/// interrupts.
pub(super) const SYSTEM_CAP: u64 = 1 << 20;

/// A file `/proc` keeps per process.
#[derive(Debug, Clone, Copy)]
pub(super) enum ProcessFile {
    Stat,
    Statm,
    Status,
    Cmdline,
}

impl ProcessFile {
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Stat => "stat",
            Self::Statm => "statm",
            Self::Status => "status",
            Self::Cmdline => "cmdline",
        }
    }

    /// How much of it is read: all of `stat` through its 23rd field however
    /// long the name, `statm`'s one line, `status` past its `Uid:` line, and
    /// as much of a command line as a row can show.
    pub(super) const fn cap(self) -> u64 {
        match self {
            Self::Stat | Self::Cmdline => 4 << 10,
            Self::Statm => 1 << 10,
            Self::Status => 16 << 10,
        }
    }
}

/// A system-wide file of `/proc`.
#[derive(Debug, Clone, Copy)]
pub(super) enum SystemFile {
    Stat,
    MemInfo,
    LoadAverage,
    Uptime,
}

impl SystemFile {
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Stat => "stat",
            Self::MemInfo => "meminfo",
            Self::LoadAverage => "loadavg",
            Self::Uptime => "uptime",
        }
    }
}

/// Whether `error` means the process exited while its files were read.
pub(super) fn vanished(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::NotFound || error.raw_os_error() == Some(ESRCH)
}

/// The one buffer every read of a sample goes through.
#[derive(Default)]
pub(super) struct Buffer(Vec<u8>);

impl Buffer {
    /// At most `cap` bytes of `path`, decoded lossily: bytes that are not
    /// UTF-8, in a name or an argument, show as U+FFFD.
    pub(super) fn read(&mut self, path: &Path, cap: u64) -> io::Result<Cow<'_, str>> {
        self.0.clear();
        File::open(path)?.take(cap).read_to_end(&mut self.0)?;
        Ok(String::from_utf8_lossy(&self.0))
    }
}
