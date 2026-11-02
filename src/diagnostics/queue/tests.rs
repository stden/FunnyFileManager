//! Slow storage must not become input latency; shutdown must drain accepted records.

use super::*;
use std::{sync::Mutex, time::Duration};

#[test]
fn a_blocked_log_sink_cannot_block_the_input_thread() {
    let (entered, waiting) = mpsc::channel();
    let (release, gate) = mpsc::channel::<()>();
    let logger = LogQueue::with_sink(move |_| {
        let _ = entered.send(()); // Only the first write has a waiting observer.
        let _ = gate.recv();
    })
    .unwrap();
    logger.record(Destination::Action, "first");
    waiting.recv_timeout(Duration::from_secs(5)).unwrap();
    let (done, result) = mpsc::channel();
    let producer = thread::spawn(move || {
        for _ in 0..CAPACITY + 5 {
            logger.record(Destination::Action, "queued navigation");
        }
        done.send(logger.dropped.load(Ordering::Relaxed)).unwrap();
    });
    let dropped = result.recv_timeout(Duration::from_secs(5));
    drop(release); // Also unblocks the writer if the assertion below fails.
    assert!(dropped.unwrap() >= 5);
    producer.join().unwrap();
}

#[test]
fn flush_preserves_record_order_destinations_and_capture_time() {
    let written = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&written);
    let logger = LogQueue::with_sink(move |record| captured.lock().unwrap().push(record)).unwrap();
    let captured_at = SystemTime::UNIX_EPOCH + Duration::from_secs(42);
    logger.record(Destination::Action, "move cursor");
    logger
        .sender
        .try_send(Message::Record(Record {
            destination: Destination::Error,
            line: "ERROR: fixture failure".into(),
            captured: captured_at,
        }))
        .unwrap();
    logger.flush();
    let records = written.lock().unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].destination, Destination::Action);
    assert_eq!(records[1].destination, Destination::Error);
    assert_eq!(records[0].line, "move cursor");
    assert_eq!(records[1].captured, captured_at);
}

#[test]
fn flush_reports_queue_overflow_without_losing_accepted_records() {
    let (sender, receiver) = mpsc::sync_channel(2);
    let logger = LogQueue {
        sender,
        dropped: Arc::default(),
    };
    logger.record(Destination::Action, "first");
    logger.record(Destination::Action, "second");
    logger.record(Destination::Action, "overflow");
    let losses = Arc::clone(&logger.dropped);
    let flusher = thread::spawn(move || logger.flush());
    let mut records = Vec::new();
    write_messages(receiver, &losses, |record| records.push(record));
    flusher.join().unwrap();
    assert_eq!(records.len(), 3);
    assert_eq!(records[0].line, "first");
    assert_eq!(records[1].line, "second");
    assert_eq!(records[2].destination, Destination::Error);
    assert!(records[2].line.contains("1 records omitted"));
}

#[test]
#[ignore = "manual storage/input latency comparison; no machine-dependent threshold"]
fn measure_log_enqueue_against_synchronous_storage() {
    let root = tempfile::TempDir::new().unwrap();
    let path = root.path().join("synchronous.log");
    let started = std::time::Instant::now();
    for _ in 0..512 {
        super::super::append_log_line_at(&path, "fixture navigation", SystemTime::now());
    }
    let synchronous = started.elapsed();
    let logger = LogQueue::start(root.path()).unwrap();
    let started = std::time::Instant::now();
    for _ in 0..512 {
        logger.record(Destination::Action, "fixture navigation");
    }
    let queued = started.elapsed();
    logger.flush();
    let written = std::fs::read_to_string(root.path().join("ffm_actions.log")).unwrap();
    assert_eq!(written.lines().count(), 512);
    eprintln!("512 records: synchronous={synchronous:?}, input enqueue={queued:?}");
}

#[test]
fn a_stopped_writer_does_not_make_logging_or_flush_fail() {
    let (sender, receiver) = mpsc::sync_channel(1);
    drop(receiver);
    let logger = LogQueue {
        sender,
        dropped: Arc::default(),
    };
    logger.record(Destination::Action, "after shutdown");
    logger.flush();
    assert_eq!(logger.dropped.load(Ordering::Relaxed), 1);
}
