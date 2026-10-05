//! Purpose: The thread-local progress screen: when it may draw, what it is showing,
//! and installing the painter that draws it.

use std::cell::RefCell;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::{Operation, Painter, Phase, Snapshot, Tally, SHOWN_EVERY};

/// When the dialog may be drawn again.
#[derive(Debug, Clone, Copy)]
pub(super) struct Throttle {
    pub(super) started: Instant,
    pub(super) last: Option<Instant>,
}

impl Throttle {
    pub(super) const fn new(started: Instant) -> Self {
        Self {
            started,
            last: None,
        }
    }

    /// Whether a frame drawn at `now` would be `interval` after the last one
    /// — or, for the first, `interval` after the operation started.
    pub(super) fn due(&self, now: Instant, interval: Duration) -> bool {
        now.saturating_duration_since(self.last.unwrap_or(self.started)) >= interval
    }
}

/// The operation being shown.
struct Shown {
    operation: Operation,
    phase: Phase,
    current: PathBuf,
    done: Tally,
    total: Tally,
    throttle: Throttle,
}

impl Shown {
    /// `operation` on `sources`, started at `now`: measuring them first, or,
    /// for a move, counting them as what it has to send (see `source_done`).
    fn new(operation: Operation, sources: &[PathBuf], now: Instant) -> Self {
        let (phase, files) = match operation {
            Operation::Copy | Operation::Delete | Operation::Find => (Phase::Scanning, 0),
            Operation::Move => (
                Phase::Working,
                u64::try_from(sources.len()).unwrap_or(u64::MAX),
            ),
        };
        Self {
            operation,
            phase,
            current: sources.first().cloned().unwrap_or_default(),
            done: Tally::default(),
            total: Tally { files, bytes: 0 },
            throttle: Throttle::new(now),
        }
    }

    fn snapshot(&self, now: Instant) -> Snapshot {
        Snapshot {
            operation: self.operation,
            phase: self.phase,
            current: self.current.clone(),
            done: self.done,
            total: self.total,
            elapsed: now.saturating_duration_since(self.throttle.started),
        }
    }
}

/// The installed painter and what it is showing.
struct Screen {
    painter: Box<dyn Painter>,
    shown: Option<Shown>,
    /// Whether the dialog was drawn since the event loop last asked.
    painted: bool,
    interval: Duration,
}

thread_local! {
    static SCREEN: RefCell<Option<Screen>> = const { RefCell::new(None) };
}

/// Keeps `painter` installed on this thread until it is dropped.
#[must_use = "the painter is uninstalled as soon as this is dropped"]
pub struct Installed(());

impl Drop for Installed {
    fn drop(&mut self) {
        SCREEN.with(|screen| screen.borrow_mut().take());
    }
}

/// Install `painter` for the operations run on this thread from now on.
pub fn install(painter: Box<dyn Painter>) -> Installed {
    install_with(painter, SHOWN_EVERY)
}

/// [`install`], drawing at most once per `interval`: a test passes zero to
/// see every report drawn without waiting for the clock.
pub fn install_with(painter: Box<dyn Painter>, interval: Duration) -> Installed {
    SCREEN.with(|screen| {
        *screen.borrow_mut() = Some(Screen {
            painter,
            shown: None,
            painted: false,
            interval,
        });
    });
    Installed(())
}

/// Whether the dialog was drawn since the last call.
///
/// The event loop repaints the whole screen when it was: the dialog was drawn
/// over the last frame, and the next frame only redraws what it thinks changed.
pub fn take_painted() -> bool {
    SCREEN.with(|screen| {
        screen
            .borrow_mut()
            .as_mut()
            .is_some_and(|screen| std::mem::take(&mut screen.painted))
    })
}

/// Shows one operation until it is dropped.
#[must_use = "the operation stops being shown as soon as this is dropped"]
pub struct Run {
    /// Whether this run is the one being shown. A run begun inside another,
    /// or with no painter, shows nothing and ends nothing.
    owns: bool,
}

impl Drop for Run {
    fn drop(&mut self) {
        if self.owns {
            with_screen(|screen| screen.shown = None);
        }
    }
}

/// Change what is shown, then draw it if the last frame is old enough.
fn update(change: impl FnOnce(&mut Shown)) {
    with_screen(|screen| {
        let Some(shown) = screen.shown.as_mut() else {
            return;
        };
        change(shown);
        let now = Instant::now();
        if !shown.throttle.due(now, screen.interval) {
            return;
        }
        shown.throttle.last = Some(now);
        let snapshot = shown.snapshot(now);
        screen.painter.paint(&snapshot);
        screen.painted = true;
    });
}

fn with_screen<T>(act: impl FnOnce(&mut Screen) -> T) -> Option<T> {
    SCREEN.with(|screen| screen.borrow_mut().as_mut().map(act))
}

mod report;

pub use report::{
    begin, copied, copying_across, entering, examined, file_copied, is_shown, matched, relay,
    removed, source_done, tick,
};
