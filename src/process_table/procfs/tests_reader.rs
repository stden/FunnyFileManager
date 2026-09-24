//! Purpose: Tests for the reader over a fake `/proc` in a temporary
//! directory — every field of a process, entries that name no process,
//! processes that vanish or cannot be read, optional files that only cost
//! their field — and the fake `/proc` the other reader tests share.

use std::fs;
use std::path::Path;
use std::time::Duration;

use tempfile::TempDir;

use super::parse::{ClockTicks, PageSize};
use super::ProcFs;
use crate::models::{
    KernelPriority, MemoryBytes, MemoryUsage, Niceness, Owner, Pid, Priority, ProcessKey,
    ProcessKind, ProcessReader, ProcessSample, ProcessState, Sample, Start, StartTime,
};

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

#[test]
fn a_process_is_read_from_its_stat_statm_status_and_cmdline() {
    let fake = FakeProc::new();
    fake.add(&Process::new(42, "bob"));
    let sample = sample(&mut fake.reader());
    let process = only(&sample);
    let kib = |amount| MemoryBytes::from_kib(amount).expect("a size");
    assert_eq!(process.key, key(42));
    assert_eq!(process.parent, Some(pid(1)));
    assert_eq!(process.name, "bob");
    assert_eq!(process.command, "/usr/bin/bob --flag");
    assert_eq!(process.owner, Owner::Uid(NOBODY_KNOWS));
    assert_eq!(process.state, ProcessState::Sleeping);
    let nice = Niceness::try_from(0).expect("a nice value");
    let kernel = KernelPriority::new(20);
    assert_eq!(process.priority, Priority::Unix { kernel, nice });
    assert_eq!(process.threads, 2);
    assert_eq!(process.cpu_time, Duration::from_secs(3));
    let memory = MemoryUsage {
        virtual_size: kib(8000),
        resident: kib(400),
        shared: Some(kib(200)),
    };
    assert_eq!(process.memory, Some(memory));
    assert_eq!(process.running_for, Some(Duration::from_millis(996_300)));
    assert_eq!(process.kind, ProcessKind::User);
}

#[test]
fn entries_that_name_no_process_are_ignored() {
    let fake = FakeProc::new();
    fake.add(&Process::new(42, "bob"));
    for entry in [
        "self/stat",
        "thread-self/stat",
        "sys/stat",
        "net/stat",
        "1a/stat",
        "+7/stat",
    ] {
        fake.write(entry, "7 (x) S 1\n");
    }
    fake.write("0/stat", "0 (x) S 1\n");
    let sample = sample(&mut fake.reader());
    assert_eq!(pids(&sample), [42]);
    assert_eq!(sample.unreadable, 0);
}

#[test]
fn a_process_directory_without_stat_has_exited_and_is_not_counted() {
    let fake = FakeProc::new();
    fake.add(&Process::new(42, "bob"));
    fake.add(&Process::new(43, "alice"));
    fake.remove("43/stat");
    let sample = sample(&mut fake.reader());
    assert_eq!((pids(&sample), sample.unreadable), (vec![42], 0));
}

/// Any per-process file gone means the process exited between two reads.
#[test]
fn a_process_whose_other_files_are_gone_has_exited_and_is_not_counted() {
    for file in ["statm", "status", "cmdline"] {
        let fake = FakeProc::new();
        fake.add(&Process::new(42, "bob"));
        fake.add(&Process::new(43, "alice"));
        fake.remove(&format!("43/{file}"));
        let sample = sample(&mut fake.reader());
        assert_eq!((pids(&sample), sample.unreadable), (vec![42], 0), "{file}");
    }
}

#[test]
fn a_stat_that_cannot_be_read_or_parsed_is_counted_as_unreadable() {
    let fake = FakeProc::new();
    fake.add(&Process::new(42, "bob"));
    fake.add(&Process::new(43, "alice"));
    fake.add(&Process::new(44, "carol"));
    fake.add(&Process::new(45, "dave"));
    fake.unreadable("43/stat");
    fake.write("44/stat", "44 (carol) S 1 44\n");
    fake.write(
        "45/stat",
        "46 (dave) S 1 45 45 0 -1 0 0 0 0 0 0 0 0 0 20 0 1 0 450 0 0\n",
    );
    let sample = sample(&mut fake.reader());
    assert_eq!((pids(&sample), sample.unreadable), (vec![42], 3));
}

#[test]
fn optional_files_that_cannot_be_read_cost_only_their_field() {
    let fake = FakeProc::new();
    fake.add(&Process::new(42, "bob"));
    for file in ["42/statm", "42/status", "42/cmdline"] {
        fake.unreadable(file);
    }
    let sample = sample(&mut fake.reader());
    let process = only(&sample);
    assert_eq!(process.memory, None);
    assert_eq!(process.owner, Owner::Unknown);
    assert_eq!(process.command, "[bob]");
    assert_eq!(sample.unreadable, 0);
}

#[test]
fn a_status_without_an_effective_uid_leaves_the_owner_unknown() {
    let fake = FakeProc::new();
    fake.add(&Process::new(42, "bob"));
    fake.write("42/status", "Name:\tbob\nUid:\t1000\n");
    assert_eq!(only(&sample(&mut fake.reader())).owner, Owner::Unknown);
}

#[test]
fn a_process_listed_twice_is_one_row() {
    let fake = FakeProc::new();
    fake.add(&Process::new(42, "bob"));
    let stat = fs::read_to_string(fake.root().join("42/stat")).expect("stat");
    for file in ["stat", "statm", "status", "cmdline"] {
        let text = fs::read_to_string(fake.root().join(format!("42/{file}"))).expect(file);
        fake.write(&format!("042/{file}"), &text);
    }
    fake.write("042/stat", &stat);
    assert_eq!(pids(&sample(&mut fake.reader())), [42]);
}
