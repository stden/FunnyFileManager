//! Purpose: Keep capture lossless when protocol replies are full or disconnected.

use super::*;
use std::{sync::mpsc, time::Duration};

#[test]
fn full_reply_queue_does_not_drop_output_or_block_capture() {
    let host = HeadlessHost::new();
    let (sender, _input) = mpsc::sync_channel(1);
    let generation = host.begin(sender).unwrap();
    let (replies, responses) = mpsc::sync_channel(1);
    let captured = Arc::new(Mutex::new(Captured::default()));
    let mut capture = Capture::new(host.clone(), generation, Arc::clone(&captured), replies);
    let bytes = b"before\x1b[6n\x1b[6nafter";
    let result = crate::test_support::run_with_timeout(Duration::from_secs(2), move || {
        capture.record(bytes, bytes.len())
    });
    let first = responses.try_recv();
    let remaining = responses.try_recv();
    drop(responses); // Release a blocked writer before reporting a regression.
    assert!(
        matches!(result, Some(Ok(()))),
        "capture must finish without blocking: {result:?}"
    );
    assert!(matches!(first, Ok(Reply::CursorPosition)));
    assert!(matches!(
        remaining,
        Err(mpsc::TryRecvError::Empty | mpsc::TryRecvError::Disconnected)
    ));
    assert_eq!(captured.lock().unwrap().bytes(), bytes);
    assert_eq!(host.snapshot().output.as_bytes(), bytes);
}

#[test]
fn disconnected_reply_queue_still_captures_and_invalid_read_is_atomic() {
    let host = HeadlessHost::new();
    let (sender, _input) = mpsc::sync_channel(1);
    let generation = host.begin(sender).unwrap();
    let (replies, responses) = mpsc::sync_channel(1);
    drop(responses);
    let captured = Arc::new(Mutex::new(Captured::default()));
    let mut capture = Capture::new(host.clone(), generation, Arc::clone(&captured), replies);
    let bytes = b"\x1b[6noutput";
    let error = capture.record(bytes, bytes.len() + 1).unwrap_err();
    assert_eq!(error.to_string(), "Invalid terminal read count");
    assert_eq!(host.snapshot().output, "");
    capture.record(bytes, bytes.len()).unwrap();
    assert_eq!(captured.lock().unwrap().bytes(), bytes);
    assert_eq!(host.snapshot().output.as_bytes(), bytes);
}
