//! Bounded process lifetime and output capture for the WSL adapter.

use std::io::Read;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

mod capture;
#[cfg(windows)]
mod launch;

use capture::{capture, ManagedProcess};
#[cfg(windows)]
pub(super) use launch::run;

const OUTPUT_LIMIT: usize = 256 * 1024;
const PIPE_CHUNK: usize = 8192;
const POLL_INTERVAL: Duration = Duration::from_millis(20);
const DRAIN_GRACE: Duration = Duration::from_millis(500);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Exit {
    Success,
    Failed(Option<i32>),
}

#[derive(Debug, Default)]
pub(super) struct StreamOutput {
    pub bytes: Vec<u8>,
    pub truncated: bool,
}

impl StreamOutput {
    fn append(&mut self, bytes: &[u8]) {
        let room = OUTPUT_LIMIT.saturating_sub(self.bytes.len());
        let retained = bytes.get(..room).unwrap_or(bytes);
        self.bytes.extend_from_slice(retained);
        self.truncated |= retained.len() < bytes.len();
    }
}

#[derive(Debug)]
pub(super) struct Captured {
    pub exit: Exit,
    pub stdout: StreamOutput,
    pub stderr: StreamOutput,
}

type Pipe = Receiver<Result<Vec<u8>, String>>;

fn read_pipe(reader: impl Read + Send + 'static) -> Result<Pipe, String> {
    // The queue is bounded as well as the retained output, even for verbose upgrades.
    let (sender, receiver) = mpsc::sync_channel(4);
    thread::Builder::new()
        .name("wsl-output".to_owned())
        .spawn(move || pump_pipe(reader, &sender))
        .map_err(|error| format!("Cannot capture WSL output: {error}"))?;
    Ok(receiver)
}

/// Drain one stream into the bounded queue until EOF or its consumer closes.
fn pump_pipe(mut reader: impl Read, sender: &mpsc::SyncSender<Result<Vec<u8>, String>>) {
    let mut buffer = [0; PIPE_CHUNK];
    loop {
        let chunk = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(length) => buffer.get(..length).map(<[u8]>::to_vec).ok_or_else(|| {
                "Cannot read WSL output: the pipe reported more bytes than it read".to_owned()
            }),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => Err(format!("Cannot read WSL output: {error}")),
        };
        // A read error is the stream's last word, whether or not it arrives.
        let failed = chunk.is_err();
        if sender.send(chunk).is_err() || failed {
            break;
        }
    }
}

#[cfg(test)]
mod tests;
