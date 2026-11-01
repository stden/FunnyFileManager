//! Purpose: Validate remote-key delivery and session transition failures.

use super::*;
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use std::sync::mpsc;

#[test]
fn releases_need_no_child_and_encoded_keys_are_one_queue_item() {
    let host = HeadlessHost::new();
    let mut release = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    release.kind = KeyEventKind::Release;
    assert_eq!(host.key(release).unwrap(), 0);
    let (sender, input) = mpsc::sync_channel(1);
    host.begin(sender).unwrap();
    assert_eq!(
        host.key(KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL))
            .unwrap(),
        6
    );
    assert_eq!(input.try_recv().unwrap(), b"\x1b[1;5A");
    assert_eq!(
        host.key(KeyEvent::new(KeyCode::CapsLock, KeyModifiers::NONE))
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput
    );
    assert!(matches!(input.try_recv(), Err(mpsc::TryRecvError::Empty)));
}

#[test]
fn closed_child_input_reports_broken_pipe_without_changing_running_state() {
    let host = HeadlessHost::new();
    let (sender, input) = mpsc::sync_channel(1);
    host.begin(sender).unwrap();
    drop(input);
    let error = host.input("private").unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    assert_eq!(error.to_string(), "External input closed");
    assert_eq!(host.snapshot().state, ExternalState::Running);
}

#[test]
fn a_second_launch_cannot_replace_a_live_input_queue() {
    let host = HeadlessHost::new();
    let (first, input) = mpsc::sync_channel(1);
    let generation = host.begin(first).unwrap();
    let (second, _) = mpsc::sync_channel(1);
    assert_eq!(
        host.begin(second).err().unwrap().kind(),
        io::ErrorKind::WouldBlock
    );
    host.input("first-child").unwrap();
    assert_eq!(input.recv().unwrap(), b"first-child");
    assert!(host.active(generation));
}

#[test]
fn cancelled_launch_remains_idle_until_reset_and_stale_completion_is_ignored() {
    let host = HeadlessHost::new();
    host.cancel();
    let (sender, _) = mpsc::sync_channel(1);
    assert_eq!(
        host.begin(sender).err().unwrap().kind(),
        io::ErrorKind::Interrupted
    );
    assert_eq!(host.snapshot().state, ExternalState::Idle);
    host.reset().unwrap();
    let (sender, _input) = mpsc::sync_channel(1);
    let generation = host.begin(sender).unwrap();
    host.finish(
        Generation(generation.0.wrapping_sub(1)),
        ExternalState::Failed,
    );
    assert!(host.active(generation));
    host.finish(generation, ExternalState::Exited { code: 0 });
    assert_eq!(host.snapshot().state, ExternalState::Exited { code: 0 });
}
