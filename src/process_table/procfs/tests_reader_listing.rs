//! Purpose: Tests for which entries of a fake `/proc` become rows: entries
//! that name no process are ignored, a process that vanishes mid-read is
//! left out without a word, one whose `stat` cannot be read or parsed is
//! counted as unreadable, and a process listed twice is one row.

use std::fs;

use super::tests_reader::{pids, sample, FakeProc, Process};

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
