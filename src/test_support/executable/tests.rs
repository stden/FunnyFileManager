//! Purpose: A fixture written and run at once still runs while other threads fork.

use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use tempfile::TempDir;

use super::write_executable;

/// Writing a file in this process and running it straight away used to fail
/// with ETXTBSY whenever a sibling thread forked in between. Four threads fork
/// in a loop while 200 fixtures are written and run.
#[test]
fn a_fixture_runs_at_once_while_other_threads_fork() {
    let stop = Arc::new(AtomicBool::new(false));
    let forkers: Vec<_> = (0..4)
        .map(|_| {
            let stop = Arc::clone(&stop);
            thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    drop(Command::new("/bin/true").status());
                }
            })
        })
        .collect();

    let dir = TempDir::new().expect("temp dir");
    let runs: Vec<_> = (0..200)
        .map(|index| {
            let program = dir.path().join(format!("program-{index}"));
            write_executable(&program, b"#!/bin/sh\nexit 0\n").expect("fixture");
            Command::new(&program).status()
        })
        .collect();

    stop.store(true, Ordering::Relaxed);
    for forker in forkers {
        forker.join().expect("forker");
    }
    for run in runs {
        assert!(run.expect("the fixture runs at once").success());
    }
}
