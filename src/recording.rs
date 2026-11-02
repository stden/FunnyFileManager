//! Portable input recordings and timed replay through the normal event loop.

use crossterm::event::{self, Event};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::VecDeque, io, path::PathBuf, rc::Rc, time::Duration};

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

mod args;
mod clipboard;
mod processes;
mod recorder;
mod replay;
mod storage;
mod tty;

pub use args::RecordingDisabled;
use recorder::record_live;
pub use recorder::{stopped_reason, Recorder};
pub use replay::Replay;
use storage::{default_path, default_root};
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
        read_live(recorder).map(Some)
    }

    /// Throw away every live event already queued, recording none of them —
    /// see [`crate::terminal::discard_queued`] — and say whether one was the
    /// cancel gesture. A replay keeps its events: they are what the session
    /// did, and it threw the others away before they were recorded.
    pub(crate) fn discard_queued(&self) -> io::Result<bool> {
        let Self::Live(_) = self else {
            return Ok(false);
        };
        tty::wait(Duration::ZERO)?;
        Ok(crate::terminal::discard_queued())
    }

    pub(crate) fn read(&mut self) -> io::Result<Option<Event>> {
        // crossterm parses a whole burst — a paste, fast typing, key repeat —
        // in one read and keeps the rest in its own queue, not on the
        // terminal. The wait below watches the terminal, so it would sit out
        // the whole poll for every queued key. Hand those out first.
        if let Some(event) = self.read_pending()? {
            return Ok(Some(event));
        }
        match self {
            Self::Replay(replay) => replay.read(),
            Self::Live(recorder) => {
                // The wait belongs here, not to crossterm: a hung-up terminal
                // traps crossterm's reader in a loop it never leaves. See `tty`.
                let unspent = tty::wait(POLL)?;
                if !event::poll(tty::crossterm_wait(POLL))? {
                    // crossterm had nothing to give. Its reader is
                    // edge-triggered, so bytes it left queued make the probe
                    // above fire at once without producing an event; asking
                    // again immediately would pin a core. Spend the rest of
                    // the budget here instead.
                    std::thread::sleep(unspent);
                    return Ok(None);
                }
                read_live(recorder).map(Some)
            }
        }
    }
}

/// Read one live event as the user typed it — see
/// [`crate::terminal::decode_input`] — and record it that way, so a replay
/// does what the session did.
fn read_live(recorder: &mut Option<Rc<RefCell<Recorder>>>) -> io::Result<Event> {
    let event = crate::terminal::decode_input(event::read()?);
    record_live(recorder, &event);
    Ok(event)
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
