//! Purpose: One sampling thread of the task list and the channels to it:
//! a request in, a sample out, one of each at a time.
//!
//! The thread holds the reader; the UI thread holds only the channel ends
//! and the handle, so dropping a worker never waits for it: an idle thread
//! ends at its next receive, and one still reading ends when its read
//! returns and finds nobody to answer.

use std::io;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use crate::models::{ProcessReader, Sample, SampleError, TaskSession};

/// The sampling thread's name, as `ps` and a debugger show it.
const THREAD_NAME: &str = "ffm-tasks";

pub(super) type Answer = Result<Sample, SampleError>;

/// A sampling thread serving one opening of the list.
pub(super) struct Worker {
    pub(super) session: TaskSession,
    pub(super) requests: SyncSender<()>,
    pub(super) results: Receiver<Answer>,
    pub(super) handle: JoinHandle<()>,
    pub(super) flight: Flight,
}

/// Whether the worker has a request to answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Flight {
    Idle,
    /// Sent at this instant.
    Since(Instant),
}

/// A worker for `session`, sampling with `reader` on a thread of its own.
///
/// Both channels hold one message, and the model asks for one sample at a
/// time, so a slow read stretches the period rather than queue requests.
pub(super) fn spawn(reader: Box<dyn ProcessReader>, session: TaskSession) -> io::Result<Worker> {
    let (requests, incoming) = mpsc::sync_channel(1);
    let (outgoing, results) = mpsc::sync_channel(1);
    let handle = thread::Builder::new()
        .name(THREAD_NAME.to_owned())
        .spawn(move || serve(reader, &incoming, &outgoing))?;
    Ok(Worker {
        session,
        requests,
        results,
        handle,
        flight: Flight::Idle,
    })
}

/// The worker's loop: a sample per request, until either side is gone.
fn serve(
    mut reader: Box<dyn ProcessReader>,
    requests: &Receiver<()>,
    results: &SyncSender<Answer>,
) {
    while requests.recv().is_ok() {
        if results.send(reader.sample()).is_err() {
            break;
        }
    }
}
