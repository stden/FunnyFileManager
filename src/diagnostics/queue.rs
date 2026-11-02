//! Bounded background log delivery; the input thread never waits on the sink.

use std::{
    io,
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender},
        Arc, OnceLock,
    },
    thread,
    time::SystemTime,
};

const CAPACITY: usize = 1_024;
static LOGGER: OnceLock<Option<LogQueue>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Destination {
    Action,
    Error,
}

#[derive(Debug)]
struct Record {
    destination: Destination,
    line: String,
    captured: SystemTime,
}

impl Record {
    fn new(destination: Destination, line: &str) -> Self {
        Self {
            destination,
            line: line.to_owned(),
            captured: SystemTime::now(),
        }
    }
}

#[derive(Debug)]
enum Message {
    Record(Record),
    Flush(mpsc::Sender<()>),
}

struct LogQueue {
    sender: SyncSender<Message>,
    dropped: Arc<AtomicU64>,
}

impl LogQueue {
    fn start(directory: &Path) -> io::Result<Self> {
        let actions = directory.join("ffm_actions.log");
        let errors = directory.join("ffm_errors.log");
        Self::with_sink(move |record| {
            let path = match record.destination {
                Destination::Action => &actions,
                Destination::Error => &errors,
            };
            super::append_log_line_at(path, &record.line, record.captured);
        })
    }

    fn with_sink(sink: impl FnMut(Record) + Send + 'static) -> io::Result<Self> {
        let (sender, receiver) = mpsc::sync_channel(CAPACITY);
        let dropped = Arc::new(AtomicU64::new(0));
        let losses = Arc::clone(&dropped);
        let worker = thread::Builder::new()
            .name("ffm-diagnostics".into())
            .spawn(move || write_messages(receiver, &losses, sink))?;
        // One process-lifetime writer; flush waits for its accepted records at exit.
        drop(worker);
        Ok(Self { sender, dropped })
    }

    fn record(&self, destination: Destination, line: &str) {
        let record = Record::new(destination, line);
        if self.sender.try_send(Message::Record(record)).is_err() {
            // Logging is best effort: slow or failed storage must never block input.
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn flush(&self) {
        let (done, received) = mpsc::channel();
        if self.sender.send(Message::Flush(done)).is_ok() {
            // A failed writer cannot change the application's exit result.
            let _ = received.recv();
        }
    }
}

fn write_messages(receiver: Receiver<Message>, dropped: &AtomicU64, mut sink: impl FnMut(Record)) {
    for message in receiver {
        match message {
            Message::Record(record) => sink(record),
            Message::Flush(done) => {
                let omitted = dropped.swap(0, Ordering::Relaxed);
                if omitted > 0 {
                    sink(Record::new(
                        Destination::Error,
                        &format!("ERROR: Diagnostic queue saturated; {omitted} records omitted"),
                    ));
                }
                // The caller may have stopped waiting; logging remains best effort.
                let _ = done.send(());
            }
        }
    }
}

pub(super) fn submit(directory: &Path, destination: Destination, line: &str) {
    // If the worker cannot start, skip diagnostics rather than doing I/O on input.
    if let Some(logger) = LOGGER.get_or_init(|| LogQueue::start(directory).ok()) {
        logger.record(destination, line);
    }
}

pub(super) fn flush() {
    if let Some(Some(logger)) = LOGGER.get() {
        logger.flush();
    }
}

#[cfg(test)]
mod tests;
