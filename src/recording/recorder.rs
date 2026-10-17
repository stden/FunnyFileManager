//! Purpose: `Recorder` — append each live event to the session file, and stop recording (never the session) when a write fails.

use crossterm::event::Event;
use serde::Serialize;
use std::{
    cell::RefCell,
    fs::{File, OpenOptions},
    io::{self, Write},
    path::Path,
    rc::Rc,
    time::Instant,
};

use super::{Header, RecordedEvent};

pub struct Recorder {
    pub(super) file: File,
    pub(super) previous: Instant,
}

impl Recorder {
    pub(super) fn create(path: &Path, header: &Header) -> io::Result<Self> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(path)?;
        file.write_all(include_bytes!("replay.py"))?;
        writeln!(file, "\n# FFM {}", serde_json::to_string(header)?)?;
        file.flush()?;
        Ok(Self {
            file,
            previous: Instant::now(),
        })
    }

    pub(super) fn record(&mut self, event: &Event) -> io::Result<()> {
        let now = Instant::now();
        let record = RecordedEvent {
            delay_ms: u64::try_from(now.duration_since(self.previous).as_millis())
                .unwrap_or(u64::MAX),
            event: event.clone(),
        };
        self.append(&record)?;
        self.previous = now;
        Ok(())
    }
}

impl Recorder {
    pub(super) fn append(&mut self, entry: &impl Serialize) -> io::Result<()> {
        writeln!(self.file, "# FFM {}", serde_json::to_string(entry)?)?;
        self.file.flush()
    }
}

/// Append one event, or stop recording. A full disk or a revoked file ends the
/// recording; it must never end the session the user is in the middle of.
pub(super) fn record_live(recorder: &mut Option<Rc<RefCell<Recorder>>>, event: &Event) {
    let failure = recorder
        .as_ref()
        .and_then(|active| active.borrow_mut().record(event).err());
    if let Some(error) = failure {
        crate::diagnostics::log_error(&format!("recording stopped: {error}"));
        // The TUI owns the screen, so this cannot be said now without drawing
        // over it. It is said at exit instead: a recording that stops halfway
        // and never mentions it is worse than one that was never started.
        STOPPED.with(|slot| *slot.borrow_mut() = Some(error.to_string()));
        *recorder = None;
    }
}

thread_local! {
    /// Why the recording stopped, for the exit to report once the terminal is
    /// the user's again. See [`stopped_reason`].
    static STOPPED: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Why a recording stopped mid-session, taken so it is reported once.
pub fn stopped_reason() -> Option<String> {
    STOPPED.with(|slot| slot.borrow_mut().take())
}
