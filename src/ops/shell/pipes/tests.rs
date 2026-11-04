//! Purpose: Unit tests for `collect_until` budget expiry and `drain` stream edge cases.

use std::io::{self, Read};
use std::sync::{mpsc, Arc, Barrier};
use std::time::{Duration, Instant};

use super::{collect_until, drain, CHUNK_SIZE};

struct OverRead;

impl Read for OverRead {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Ok(CHUNK_SIZE + 1)
    }
}

struct TwoChunks {
    step: usize,
    dropped: Arc<Barrier>,
}

impl Read for TwoChunks {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.step += 1;
        match self.step {
            1 => {
                buf[0] = b'a';
                Ok(1)
            }
            2 => {
                self.dropped.wait();
                buf[0] = b'b';
                Ok(1)
            }
            _ => Ok(0),
        }
    }
}

#[test]
fn collect_until_returns_immediately_when_deadline_already_passed() {
    let (_tx, rx) = mpsc::channel();
    let expired = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();
    let out = collect_until(&rx, expired);
    assert_eq!(out, b"");
}

#[test]
fn drain_stops_cleanly_when_reader_exceeds_buffer_contract() {
    let rx = drain(Some(OverRead));
    assert!(rx.recv_timeout(Duration::from_secs(2)).is_err());
}

#[test]
fn drain_stops_when_receiver_is_dropped_mid_stream() {
    let dropped = Arc::new(Barrier::new(2));
    let rx = drain(Some(TwoChunks {
        step: 0,
        dropped: Arc::clone(&dropped),
    }));
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(2)).expect("first"),
        b"a"
    );
    drop(rx);
    dropped.wait();
}
