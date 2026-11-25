//! Unit tests for `SystemClipboard` error propagation on OSC 52 writes and flushes.

use std::io;

use super::*;

struct FailingWrite;

impl Write for FailingWrite {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("write failed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct FailingFlush;

impl Write for FailingFlush {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("flush failed"))
    }
}

#[test]
fn emit_osc52_reports_write_and_flush_errors() {
    let mut write_err = SystemClipboard::terminal_only(Box::new(FailingWrite));
    let err = write_err
        .emit_osc52("\x1b]52;c;YQ==\x07")
        .expect_err("write failure must surface");
    assert!(err.contains("write failed"), "{err}");

    let mut flush_err = SystemClipboard::terminal_only(Box::new(FailingFlush));
    let err = flush_err
        .emit_osc52("\x1b]52;c;YQ==\x07")
        .expect_err("flush failure must surface");
    assert!(err.contains("flush failed"), "{err}");
}
