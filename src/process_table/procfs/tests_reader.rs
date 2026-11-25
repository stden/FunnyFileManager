//! Purpose: The fake `/proc` the reader tests share: the system files and a
//! directory per fixture process in a temporary directory, and the helpers
//! that sample it and pick the result apart. The tests of the rows are in
//! `tests_reader_fields.rs` and `tests_reader_listing.rs`.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::parse::{ClockTicks, PageSize};
use super::ProcFs;
use crate::models::{Pid, ProcessKey, ProcessReader, ProcessSample, Sample, Start, StartTime};

/// A uid no passwd database has an entry for.
pub(super) const NOBODY_KNOWS: u32 = 4_000_000_000;

const PROC_STAT: &str = "cpu  100 20 30 400 50 6 7 8 9 10\ncpu0 100 20 30 400 50 6 7 8 9 10\n\
    intr 12345 0 0 0\nctxt 99\nprocs_running 3\n";
const MEMINFO: &str = "MemTotal:  1000 kB\nMemFree:  400 kB\nMemAvailable:  600 kB\n\
    SwapTotal:  200 kB\nSwapFree:  150 kB\n";

/// A fake `/proc`: the system files, and a directory per process whose
/// files end their lines with a newline, as the kernel writes them.
pub(super) struct FakeProc(TempDir);

/// One fixture process: sleeping, started at tick `pid * 10`, with 2.5 s of
/// user and 0.5 s of system time, owned by [`NOBODY_KNOWS`].
pub(super) struct Process {
    pub(super) pid: u32,
    pub(super) name: &'static str,
    pub(super) state: char,
    pub(super) command: &'static str,
}

impl Process {
    pub(super) const fn new(pid: u32, name: &'static str) -> Self {
        Self {
            pid,
            name,
            state: 'S',
            command: "/usr/bin/bob\0--flag\0",
        }
    }
}

impl FakeProc {
    pub(super) fn new() -> Self {
        let fake = Self(TempDir::new().expect("temporary directory"));
        fake.write("stat", PROC_STAT);
        fake.write("meminfo", MEMINFO);
        fake.write("loadavg", "0.52 0.58 0.59 3/120 4242\n");
        fake.write("uptime", "1000.50 800.00\n");
        fake
    }

    pub(super) fn root(&self) -> &Path {
        self.0.path()
    }

    pub(super) fn write(&self, relative: &str, text: &str) {
        let path = self.root().join(relative);
        fs::create_dir_all(path.parent().expect("a parent")).expect("parent directory");
        fs::write(path, text).expect("fixture file");
    }

    /// `relative` as a directory, which every read of it fails on, whoever
    /// runs the test: root can read any file, but not a directory as one.
    pub(super) fn unreadable(&self, relative: &str) {
        let path = self.root().join(relative);
        drop(fs::remove_file(&path));
        fs::create_dir_all(path).expect("a directory in the file's place");
    }

    pub(super) fn remove(&self, relative: &str) {
        fs::remove_file(self.root().join(relative)).expect("remove fixture file");
    }

    pub(super) fn add(&self, process: &Process) {
        let Process {
            pid, name, state, ..
        } = process;
        let start = pid * 10;
        self.write(
            &format!("{pid}/stat"),
            &format!(
                "{pid} ({name}) {state} 1 {pid} {pid} 0 -1 4194560 0 0 0 0 250 50 0 0 20 0 \
                 2 0 {start} 8192000 100 18446744073709551615\n"
            ),
        );
        self.write(&format!("{pid}/statm"), "2000 100 50 1 0 80 0\n");
        self.write(
            &format!("{pid}/status"),
            &format!("Name:\t{name}\nUid:\t1000\t{NOBODY_KNOWS}\t1000\t1000\n"),
        );
        self.write(&format!("{pid}/cmdline"), process.command);
    }

    pub(super) fn reader(&self) -> ProcFs {
        ProcFs::new(self.root().to_path_buf(), hertz(), page())
    }
}

pub(super) fn hertz() -> ClockTicks {
    ClockTicks::new(100).expect("USER_HZ is not zero")
}

pub(super) fn page() -> PageSize {
    PageSize::new(4096).expect("a page size")
}

pub(super) fn pid(raw: u32) -> Pid {
    Pid::new(raw).expect("a fixture PID")
}

pub(super) fn key(raw: u32) -> ProcessKey {
    ProcessKey::new(pid(raw), Start::Known(StartTime::new(u64::from(raw) * 10)))
}

pub(super) fn sample(reader: &mut ProcFs) -> Sample {
    reader
        .sample()
        .unwrap_or_else(|error| panic!("a sample: {error}"))
}

pub(super) fn pids(sample: &Sample) -> Vec<u32> {
    sample
        .processes
        .iter()
        .map(|process| process.key.pid().get())
        .collect()
}

pub(super) fn only(sample: &Sample) -> &ProcessSample {
    match sample.processes.as_slice() {
        [process] => process,
        other => panic!("one process, not {}", other.len()),
    }
}

pub(super) fn commands(sample: &Sample) -> Vec<&str> {
    sample
        .processes
        .iter()
        .map(|process| process.command.as_str())
        .collect()
}
