//! Purpose: Tests for the process host a session gets: a live session, with
//! or without a recording, controls processes through the real host; a
//! replay reads through it and signals and renices nothing.

use std::sync::Arc;

use tempfile::TempDir;

use super::super::{Header, Input, Replay};
use crate::models::{ControlError, Niceness, Pid, PriorityChange, Signal};
use crate::test_support::processes::{one_core, process, Control, FixtureHost};

fn firefox() -> crate::models::ProcessSample {
    process(Pid::new(4242).expect("a PID"), "firefox")
}

/// A replay of a recording that holds only its header.
fn replay(dir: &TempDir) -> Input {
    let path = dir.path().join("events.jsonl");
    let header = Header {
        version: 1,
        executable: dir.path().join("ffm"),
        directory: dir.path().to_path_buf(),
        size: (80, 24),
    };
    let text = serde_json::to_string(&header).expect("header");
    std::fs::write(&path, text).expect("recording");
    Input::Replay(Replay::load(&path, (80, 24)).expect("replay"))
}

#[test]
fn a_replay_samples_through_the_host_but_never_signals_or_renices() {
    let dir = TempDir::new().expect("dir");
    let fixture = FixtureHost::new(vec![one_core(vec![firefox()])]);
    let host = replay(&dir).process_host(Arc::new(fixture.clone()));
    let key = firefox().key;
    assert_eq!(host.signal(key, Signal::Kill), Err(ControlError::ReadOnly));
    assert_eq!(
        host.set_priority(key, PriorityChange::Nice(Niceness::MAX)),
        Err(ControlError::ReadOnly)
    );
    assert_eq!(fixture.controls().len(), 0, "nothing reached the host");
    let sample = host.reader().expect("a reader").sample().expect("a sample");
    assert_eq!(sample.processes.len(), 1);
    assert_eq!(fixture.readers_opened(), 1);
}

#[test]
fn a_live_session_controls_processes_through_the_host_itself() {
    let dir = TempDir::new().expect("dir");
    let recorded = Input::open_recorder(&dir.path().join("session.py"), (80, 24)).expect("record");
    for input in [Input::Live(None), recorded] {
        let fixture = FixtureHost::new(Vec::new());
        let host = input.process_host(Arc::new(fixture.clone()));
        let key = firefox().key;
        host.signal(key, Signal::Hangup).expect("recorded");
        assert_eq!(fixture.controls(), [Control::Signal(key, Signal::Hangup)]);
    }
}
