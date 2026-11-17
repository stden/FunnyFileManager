//! Purpose: Preserve input through short writes and reject failed writer contracts.

use super::super::*;
use std::{collections::VecDeque, sync::mpsc};

#[derive(Default)]
struct Writer {
    steps: VecDeque<io::Result<usize>>,
    bytes: Vec<u8>,
    flush_error: Option<io::ErrorKind>,
    flushes: usize,
}

impl Write for Writer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let count = self.steps.pop_front().unwrap_or(Ok(bytes.len()))?;
        self.bytes.extend(bytes.iter().take(count).copied());
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flushes += 1;
        match self.flush_error {
            Some(kind) => Err(io::Error::from(kind)),
            None => Ok(()),
        }
    }
}

fn active() -> io::Result<(HeadlessHost, Generation, Receiver<Vec<u8>>)> {
    let host = HeadlessHost::new();
    let (sender, input) = mpsc::sync_channel(1);
    let generation = host.begin(sender)?;
    Ok((host, generation, input))
}

#[test]
fn retries_and_short_writes_deliver_each_byte_once_before_flushing() {
    let (host, generation, _input) = active().unwrap();
    let mut writer = Writer {
        steps: VecDeque::from([
            Err(io::Error::from(io::ErrorKind::Interrupted)),
            Ok(2),
            Err(io::Error::from(io::ErrorKind::WouldBlock)),
            Ok(1),
        ]),
        ..Writer::default()
    };
    deliver(&mut writer, b"abcdef", &host, generation).unwrap();
    assert_eq!(writer.bytes, b"abcdef");
    assert_eq!(writer.flushes, 1);
}

#[test]
fn zero_invalid_and_hard_writes_fail_before_flushing() {
    for (step, kind) in [
        (Ok(0), io::ErrorKind::WriteZero),
        (Ok(4), io::ErrorKind::Other),
        (
            Err(io::Error::from(io::ErrorKind::BrokenPipe)),
            io::ErrorKind::BrokenPipe,
        ),
    ] {
        let (host, generation, _input) = active().unwrap();
        let mut writer = Writer {
            steps: VecDeque::from([step]),
            ..Writer::default()
        };
        assert_eq!(
            deliver(&mut writer, b"abc", &host, generation)
                .unwrap_err()
                .kind(),
            kind
        );
        assert_eq!(writer.flushes, 0);
    }
}

#[test]
fn flush_failure_is_reported_after_all_input_is_written() {
    let (host, generation, _input) = active().unwrap();
    let mut writer = Writer {
        flush_error: Some(io::ErrorKind::BrokenPipe),
        ..Writer::default()
    };
    assert_eq!(
        deliver(&mut writer, b"abc", &host, generation)
            .unwrap_err()
            .kind(),
        io::ErrorKind::BrokenPipe
    );
    assert_eq!(writer.bytes, b"abc");
    assert_eq!(writer.flushes, 1);
}

#[test]
fn cancelled_or_stale_input_never_reaches_the_writer() {
    for cancelled in [false, true] {
        let (host, generation, _input) = active().unwrap();
        if cancelled {
            host.cancel();
        } else {
            host.finish(generation, super::super::super::super::ExternalState::Idle);
        }
        let mut writer = Writer::default();
        assert_eq!(
            deliver(&mut writer, b"abc", &host, generation)
                .unwrap_err()
                .kind(),
            io::ErrorKind::Interrupted
        );
        assert_eq!(writer.bytes.len(), 0);
        assert_eq!(writer.flushes, 0);
    }
}
