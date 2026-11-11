//! Purpose: Tests for what the reader keeps between samples: command lines
//! reused while a process keeps its name and identity, never read while it
//! sleeps in the kernel uncached, dropped with the process; and owner names,
//! with a failed lookup asked again every sample.

use std::fs;

use super::tests_reader::{commands, only, pid, pids, sample, FakeProc, Process};
use crate::models::{ProcessKey, Start, StartTime};

#[test]
fn an_unchanged_name_reuses_the_command_line_read_before() {
    let fake = FakeProc::new();
    fake.add(&Process::new(42, "bob"));
    let mut reader = fake.reader();
    sample(&mut reader);
    fake.write("42/cmdline", "/usr/bin/changed\0");
    assert_eq!(commands(&sample(&mut reader)), ["/usr/bin/bob --flag"]);
}

/// `execve` changes the short name, so the new program's arguments are read.
#[test]
fn a_changed_name_reads_the_command_line_again() {
    let fake = FakeProc::new();
    fake.add(&Process::new(42, "bob"));
    let mut reader = fake.reader();
    sample(&mut reader);
    fake.add(&Process {
        command: "/usr/bin/alice\0",
        ..Process::new(42, "alice")
    });
    assert_eq!(commands(&sample(&mut reader)), ["/usr/bin/alice"]);
}

/// Reading `cmdline` waits on the process's mmap lock, so a process asleep
/// in the kernel is shown by name until it wakes.
#[test]
fn a_process_in_disk_sleep_is_not_read_until_it_wakes() {
    let fake = FakeProc::new();
    fake.add(&Process {
        state: 'D',
        ..Process::new(42, "bob")
    });
    let mut reader = fake.reader();
    assert_eq!(commands(&sample(&mut reader)), ["[bob]"]);
    fake.add(&Process::new(42, "bob"));
    assert_eq!(commands(&sample(&mut reader)), ["/usr/bin/bob --flag"]);
}

#[test]
fn a_process_in_disk_sleep_keeps_the_command_line_read_before() {
    let fake = FakeProc::new();
    fake.add(&Process::new(42, "bob"));
    let mut reader = fake.reader();
    sample(&mut reader);
    fake.add(&Process {
        state: 'D',
        command: "/usr/bin/changed\0",
        ..Process::new(42, "bob")
    });
    assert_eq!(commands(&sample(&mut reader)), ["/usr/bin/bob --flag"]);
}

/// A process missing from one sample loses its cached command line, so the
/// cache never outgrows the table: back in the next, it is read afresh.
#[test]
fn a_process_missing_from_a_sample_is_read_again_when_it_returns() {
    let fake = FakeProc::new();
    fake.add(&Process::new(42, "bob"));
    let mut reader = fake.reader();
    sample(&mut reader);
    fake.remove("42/stat");
    assert_eq!(pids(&sample(&mut reader)).len(), 0);
    fake.add(&Process {
        command: "/usr/bin/changed\0",
        ..Process::new(42, "bob")
    });
    assert_eq!(commands(&sample(&mut reader)), ["/usr/bin/changed"]);
}

/// The same PID with a new start time is a new process with a key of its
/// own; nothing kept for the old one carries over.
#[test]
fn a_reused_pid_is_read_as_a_new_process() {
    let fake = FakeProc::new();
    fake.add(&Process::new(42, "bob"));
    let mut reader = fake.reader();
    sample(&mut reader);
    let stat = fs::read_to_string(fake.root().join("42/stat")).expect("stat");
    fake.write("42/stat", &stat.replace(" 420 ", " 999 "));
    fake.write("42/cmdline", "/usr/bin/newcomer\0");
    let sample = sample(&mut reader);
    assert_eq!(commands(&sample), ["/usr/bin/newcomer"]);
    let newcomer = ProcessKey::new(pid(42), Start::Known(StartTime::new(999)));
    assert_eq!(only(&sample).key, newcomer);
}

#[cfg(unix)]
#[test]
fn the_effective_uid_zero_is_root() {
    use crate::models::Owner;

    let fake = FakeProc::new();
    fake.add(&Process::new(42, "bob"));
    fake.write("42/status", "Uid:\t1000\t0\t1000\t1000\n");
    let owner = only(&sample(&mut fake.reader())).owner.clone();
    assert_eq!(owner, Owner::Named("root".to_owned()));
}

/// A lookup that failed (an NSS or LDAP outage) is asked again on the next
/// sample, rather than leaving a bare number for the whole session.
#[cfg(unix)]
#[test]
fn a_failed_user_lookup_is_asked_again_every_sample() {
    use super::ProcFs;
    use crate::directory::owner::Owners;
    use crate::models::Owner;

    let fake = FakeProc::new();
    fake.add(&Process::new(42, "bob"));
    fake.write("42/status", "Uid:\t0\t0\t0\t0\n");
    let mut reader = ProcFs {
        users: Owners::with_failed_user(0),
        ..fake.reader()
    };
    let owner = only(&sample(&mut reader)).owner.clone();
    assert_eq!(owner, Owner::Named("root".to_owned()));
}
