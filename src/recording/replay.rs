//! Purpose: Timed playback of a recorded session through the normal event loop.

use crossterm::event::Event;
use std::{
    collections::VecDeque,
    fs::File,
    io::{self, BufRead, BufReader},
    path::Path,
    time::{Duration, Instant},
};

use super::{ClipboardReads, Entry, Header, RecordedEvent, POLL};

pub struct Replay {
    pub(super) events: VecDeque<RecordedEvent>,
    due: Instant,
    pub(super) clipboard: ClipboardReads,
    pub(super) size: (u16, u16),
}

impl Replay {
    pub(super) fn load(path: &Path, size: (u16, u16)) -> io::Result<Self> {
        let mut lines = BufReader::new(File::open(path)?).lines();
        let header: Header = serde_json::from_str(
            &lines
                .next()
                .ok_or_else(|| io::Error::other("Empty recording"))??,
        )?;
        if header.version != 1 {
            return Err(io::Error::other("Unsupported recording version"));
        }
        let replay = Self::from_entries(lines, header.size)?;
        replay.check_size(size)?;
        Ok(replay)
    }

    fn from_entries(
        lines: impl Iterator<Item = io::Result<String>>,
        size: (u16, u16),
    ) -> io::Result<Self> {
        let mut events = VecDeque::new();
        let clipboard = ClipboardReads::default();
        for line in lines {
            match serde_json::from_str::<Entry>(&line?)? {
                Entry::Input(event) => events.push_back(event),
                Entry::Clipboard { clipboard: result } => clipboard.borrow_mut().push_back(result),
            }
        }
        let due = Instant::now()
            + Duration::from_millis(events.front().map_or(0, |entry| entry.delay_ms));
        Ok(Self {
            events,
            due,
            clipboard,
            size,
        })
    }

    pub(super) fn read(&mut self) -> io::Result<Option<Event>> {
        let remaining = self.due.saturating_duration_since(Instant::now());
        if !remaining.is_zero() {
            std::thread::sleep(remaining.min(POLL));
            return Ok(None);
        }
        let Some(entry) = self.events.pop_front() else {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Recording ended before the application quit",
            ));
        };
        self.due = Instant::now()
            + Duration::from_millis(self.events.front().map_or(0, |next| next.delay_ms));
        Ok(Some(entry.event))
    }
}

impl Replay {
    fn check_size(&self, available: (u16, u16)) -> io::Result<()> {
        let required = self
            .events
            .iter()
            .fold(self.size, |(width, height), entry| match entry.event {
                Event::Resize(w, h) => (width.max(w), height.max(h)),
                _ => (width, height),
            });
        if required.0 > available.0 || required.1 > available.1 {
            return Err(io::Error::other(format!(
                "Replay requires terminal size at least {}x{}",
                required.0, required.1
            )));
        }
        Ok(())
    }
}
