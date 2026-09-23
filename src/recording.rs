//! Portable input recordings and timed replay through the normal event loop.

use crossterm::event::{self, Event};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::VecDeque,
    fs::{File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, Instant},
};

const POLL: Duration = Duration::from_millis(100);

#[derive(Serialize, Deserialize)]
struct Header {
    version: u8,
    executable: PathBuf,
    directory: PathBuf,
    size: (u16, u16),
}

#[derive(Serialize, Deserialize)]
struct RecordedEvent {
    delay_ms: u64,
    event: Event,
}

mod clipboard;
mod replay;
mod storage;
mod tty;

pub use replay::Replay;
use storage::default_path;
#[cfg(test)]
use storage::default_root;
// Only the Unix permission test calls this; on Windows it would be unused.
#[cfg(all(test, unix))]
use storage::create_private_dir;
pub use tty::{wait as wait_for_terminal, TerminalLost};

type ClipboardReads = Rc<RefCell<VecDeque<Result<String, String>>>>;

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum Entry {
    Input(RecordedEvent),
    Clipboard { clipboard: Result<String, String> },
}

pub enum Input {
    Live(Option<Rc<RefCell<Recorder>>>),
    Replay(Replay),
}

pub struct Recorder {
    file: File,
    previous: Instant,
}

impl Recorder {
    fn create(path: &Path, header: &Header) -> io::Result<Self> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(path)?;
        file.write_all(include_bytes!("recording/replay.py"))?;
        writeln!(file, "\n# FFM {}", serde_json::to_string(header)?)?;
        file.flush()?;
        Ok(Self {
            file,
            previous: Instant::now(),
        })
    }

    fn record(&mut self, event: &Event) -> io::Result<()> {
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
    fn append(&mut self, entry: &impl Serialize) -> io::Result<()> {
        writeln!(self.file, "# FFM {}", serde_json::to_string(entry)?)?;
        self.file.flush()
    }
}

impl Input {
    /// Read only already queued live events. Replay keeps its recorded cadence.
    pub(crate) fn read_pending(&mut self) -> io::Result<Option<Event>> {
        let Self::Live(recorder) = self else {
            return Ok(None);
        };
        tty::wait(Duration::ZERO)?;
        if !event::poll(Duration::ZERO)? {
            return Ok(None);
        }
        let event = event::read()?;
        record_live(recorder, &event);
        Ok(Some(event))
    }

    pub(crate) fn from_args(args: &[std::ffi::OsString], size: (u16, u16)) -> io::Result<Self> {
        match args {
            // Recording is opt-in. A session file holds every character that
            // was typed or pasted, so a plain launch writes none — the same
            // rule the diagnostics log already follows.
            [] => Ok(Self::Live(None)),
            [flag] if flag == "--no-record" => Ok(Self::Live(None)),
            [flag] if flag == "--record" => Ok(Self::recording(default_path(), size)),
            [flag, path] if flag == "--record" => {
                Ok(Self::recording(Ok(PathBuf::from(path)), size))
            }
            [flag, path] if flag == "--replay" => {
                Replay::load(Path::new(path), size).map(Self::Replay)
            }
            _ => Err(io::Error::other(
                "Usage: ffm [--record [session.py] | --no-record | --replay events.jsonl]",
            )),
        }
    }

    /// Start recording, or say why not and carry on live. A recording is a
    /// diagnostic, never the job: a directory owned by another user, or a full
    /// disk, must not be what stops the file manager from starting.
    fn recording(path: io::Result<PathBuf>, size: (u16, u16)) -> Self {
        match path.and_then(|path| Self::open_recorder(path, size)) {
            Ok(input) => input,
            Err(error) => {
                eprintln!("ffm: recording disabled: {error}");
                Self::Live(None)
            }
        }
    }

    fn open_recorder(path: PathBuf, size: (u16, u16)) -> io::Result<Self> {
        let header = Header {
            version: 1,
            executable: std::env::current_exe()?,
            directory: std::env::current_dir()?,
            size,
        };
        let recorder = Recorder::create(&path, &header)?;
        eprintln!("Recording actions: {}", path.display());
        Ok(Self::Live(Some(Rc::new(RefCell::new(recorder)))))
    }

    pub(crate) fn read(&mut self) -> io::Result<Option<Event>> {
        match self {
            Self::Replay(replay) => replay.read(),
            Self::Live(recorder) => {
                // The wait belongs here, not to crossterm: a hung-up terminal
                // traps crossterm's reader in a loop it never leaves. See `tty`.
                let unspent = tty::wait(POLL)?;
                if !event::poll(Duration::ZERO)? {
                    // crossterm had nothing to give. Its reader is
                    // edge-triggered, so bytes it left queued make the probe
                    // above fire at once without producing an event; asking
                    // again immediately would pin a core. Spend the rest of
                    // the budget here instead.
                    std::thread::sleep(unspent);
                    return Ok(None);
                }
                let event = event::read()?;
                record_live(recorder, &event);
                Ok(Some(event))
            }
        }
    }
}

/// Append one event, or stop recording. A full disk or a revoked file ends the
/// recording; it must never end the session the user is in the middle of.
fn record_live(recorder: &mut Option<Rc<RefCell<Recorder>>>, event: &Event) {
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
    static STOPPED: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// Why a recording stopped mid-session, taken so it is reported once.
pub fn stopped_reason() -> Option<String> {
    STOPPED.with(|slot| slot.borrow_mut().take())
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_cli;
#[cfg(test)]
mod tests_failure;

impl Input {
    pub(crate) fn terminal_options(&self) -> ratatui::TerminalOptions {
        let viewport = match self {
            Self::Live(_) => ratatui::Viewport::Fullscreen,
            Self::Replay(replay) => ratatui::Viewport::Fixed(ratatui::layout::Rect::new(
                0,
                0,
                replay.size.0,
                replay.size.1,
            )),
        };
        ratatui::TerminalOptions { viewport }
    }
}
