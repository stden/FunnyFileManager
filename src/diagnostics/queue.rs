//! Bounded background log delivery; the input thread never waits on the sink.

use std::{
    io::{self, Write},
    path::{Path, PathBuf},
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

trait LogSink {
    fn write(&mut self, record: Record);
    fn flush(&mut self);
}

impl<F: FnMut(Record)> LogSink for F {
    fn write(&mut self, record: Record) {
        self(record);
    }

    fn flush(&mut self) {}
}

struct FileSink {
    actions_path: PathBuf,
    errors_path: PathBuf,
    actions: Option<io::BufWriter<std::fs::File>>,
    errors: Option<io::BufWriter<std::fs::File>>,
}

impl FileSink {
    fn new(directory: &Path) -> Self {
        Self {
            actions_path: directory.join("ffm_actions.log"),
            errors_path: directory.join("ffm_errors.log"),
            actions: None,
            errors: None,
        }
    }
}

impl LogSink for FileSink {
    fn write(&mut self, record: Record) {
        let (path, slot) = match record.destination {
            Destination::Action => (&self.actions_path, &mut self.actions),
            Destination::Error => (&self.errors_path, &mut self.errors),
        };
        if slot.is_none() {
            *slot = super::open_log_file(path).map(io::BufWriter::new);
        }
        if let Some(writer) = slot.as_mut() {
            super::write_log_line(writer, &record.line, record.captured);
        }
    }

    fn flush(&mut self) {
        if let Some(writer) = self.actions.as_mut() {
            drop(writer.flush());
        }
        if let Some(writer) = self.errors.as_mut() {
            drop(writer.flush());
        }
    }
}

struct LogQueue {
    sender: SyncSender<Message>,
    dropped: Arc<AtomicU64>,
}

impl LogQueue {
    fn start(directory: &Path) -> io::Result<Self> {
        Self::with_sink(FileSink::new(directory))
    }

    fn with_sink(sink: impl LogSink + Send + 'static) -> io::Result<Self> {
        let (sender, receiver) = mpsc::sync_channel(CAPACITY);
        let dropped = Arc::new(AtomicU64::new(0));
        let losses = Arc::clone(&dropped);
        let worker = thread::Builder::new()
            .name("ffm-diagnostics".into())
            .spawn(move || write_messages(&receiver, &losses, sink))?;
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
            received.recv().unwrap_or(());
        }
    }
}

fn write_messages(receiver: &Receiver<Message>, dropped: &AtomicU64, mut sink: impl LogSink) {
    while let Ok(first) = receiver.recv() {
        let mut current = Some(first);
        while let Some(message) = current {
            handle_message(message, dropped, &mut sink);
            current = receiver.try_recv().ok();
        }
        sink.flush();
    }
}

fn handle_message(message: Message, dropped: &AtomicU64, sink: &mut impl LogSink) {
    match message {
        Message::Record(record) => sink.write(record),
        Message::Flush(done) => {
            let omitted = dropped.swap(0, Ordering::Relaxed);
            if omitted > 0 {
                sink.write(Record::new(
                    Destination::Error,
                    &format!("ERROR: Diagnostic queue saturated; {omitted} records omitted"),
                ));
            }
            sink.flush();
            // The caller may have stopped waiting; logging remains best effort.
            done.send(()).unwrap_or(());
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
