//! Purpose: Read a child's pipes on background threads, chunk by chunk, so a
//! caller can stop waiting at any moment and keep what already arrived.

use std::io::{self, Read, Write};
use std::sync::mpsc::{self, Receiver};
use std::time::Instant;

/// Pipe read granularity. Output is forwarded in chunks rather than read to
/// EOF, so an abandoned reader still leaves everything it already sent usable.
const CHUNK_SIZE: usize = 8 * 1024;

/// Gather forwarded chunks until the reader finishes or `deadline` passes.
pub(in crate::ops) fn collect_until(chunks: &Receiver<Vec<u8>>, deadline: Instant) -> Vec<u8> {
    let mut collected = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        // `recv_timeout(ZERO)` would keep succeeding against a fast writer, so
        // an expired budget ends the collection outright.
        if remaining.is_zero() {
            return collected;
        }
        // Err is either Disconnected (the reader finished — the common case) or
        // Timeout; both mean there is nothing more to wait for.
        let Ok(chunk) = chunks.recv_timeout(remaining) else {
            return collected;
        };
        collected.extend_from_slice(&chunk);
    }
}

/// Read a child pipe on a background thread, forwarding it chunk by chunk.
///
/// Chunks rather than `read_to_end` so [`collect_until`] can stop waiting at any
/// point and still keep what already arrived. A read error ends the stream,
/// exactly like `Command::output` would. The thread also stops as soon as the
/// receiver is gone, so an abandoned reader costs one blocked read, not a
/// permanently spinning thread.
pub(super) fn drain<R: Read + Send + 'static>(source: Option<R>) -> Receiver<Vec<u8>> {
    drain_echoing(source, io::sink())
}

/// [`drain`], also copying each chunk to `echo` as it arrives, so a program
/// that owns the terminal still shows its messages live while the caller gets
/// to keep them.
pub(in crate::ops) fn drain_echoing<R, W>(source: Option<R>, mut echo: W) -> Receiver<Vec<u8>>
where
    R: Read + Send + 'static,
    W: Write + Send + 'static,
{
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let Some(mut source) = source else { return };
        let mut chunk = vec![0_u8; CHUNK_SIZE];
        while let Ok(read) = source.read(&mut chunk) {
            if read == 0 {
                return;
            }
            // `read` never exceeds the buffer (the `Read` contract); a reader
            // that broke it ends the stream like a read error would.
            let Some(bytes) = chunk.get(..read) else {
                return;
            };
            // Best effort: a lost echo leaves the kept copy intact.
            drop(echo.write_all(bytes).and_then(|()| echo.flush()));
            if sender.send(bytes.to_vec()).is_err() {
                return;
            }
        }
    });
    receiver
}
