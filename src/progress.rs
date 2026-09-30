//! Purpose: What a long copy, move or delete has done so far.
//!
//! F5, F6 and F8 report here as they go, and the painter the session
//! installed draws the progress dialog no more often than every
//! [`SHOWN_EVERY`], so the file name in it changes at most ten times a second.
//!
//! Ambient, like the fault injection in `ops::file`: the reports come from
//! deep inside the folder walk and the file copy, and threading a sink through
//! every signature between `copy_with` and `copy_file` would change a dozen of
//! them for one side effect. Everything is thread-local — the operations run
//! on the UI thread — and a report is one thread-local read while nothing is
//! being shown: no painter (every test, unless it installs one) or no
//! operation running.
//!
//! The painter draws the dialog alone, over whatever the last frame left on
//! the screen, because the operation holds `&mut App` while it runs. Once an
//! operation that painted is over, [`take_painted`] tells the event loop to
//! repaint the whole screen.

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How long the dialog shows one state before the next may replace it, and
/// how long an operation runs before the dialog first appears: one that is
/// over sooner is never shown at all.
pub const SHOWN_EVERY: Duration = Duration::from_millis(100);

/// Which operation the dialog reports on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Operation {
    Copy,
    Move,
    Delete,
}

impl Operation {
    /// The operation an F5/F6 dialog's purpose names.
    #[must_use]
    pub const fn of(purpose: crate::copy::Purpose) -> Self {
        match purpose {
            crate::copy::Purpose::Copy => Self::Copy,
            crate::copy::Purpose::Move => Self::Move,
        }
    }

    /// Whether the sources are measured before the work starts. A move is
    /// mostly renames, which take no time whatever their size, so only what
    /// has to be copied across filesystems is measured, when it comes to it.
    const fn scans_first(self) -> bool {
        match self {
            Self::Copy | Self::Delete => true,
            Self::Move => false,
        }
    }
}

/// Where an operation is: measuring what it has to do, or doing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Phase {
    Scanning,
    Working,
}

/// A count of files and of their bytes. Folders are not counted: they take
/// no time to make or remove, and a tree of empty folders is done when its
/// last one is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Tally {
    pub files: u64,
    pub bytes: u64,
}

/// One state of an operation, as the dialog shows it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    pub operation: Operation,
    pub phase: Phase,
    /// The file or folder being worked on (or measured).
    pub current: PathBuf,
    pub done: Tally,
    /// What there is to do, as far as it is known: while scanning, what has
    /// been found so far.
    pub total: Tally,
    pub elapsed: Duration,
}

impl Snapshot {
    /// How far along the operation is, in thousandths: by bytes once there
    /// are bytes to count, by files otherwise. Never above 1000, although a
    /// tree can grow while it is copied.
    #[must_use]
    pub fn permille(&self) -> u16 {
        let (done, total) = if self.total.bytes > 0 {
            (self.done.bytes, self.total.bytes)
        } else {
            (self.done.files, self.total.files)
        };
        if total == 0 {
            return 0;
        }
        let permille = (u128::from(done) * 1000 / u128::from(total)).min(1000);
        u16::try_from(permille).unwrap_or(1000)
    }
}

/// Draws the progress dialog. The session installs the one that draws on
/// the terminal; tests install one that records what it was asked to draw.
pub trait Painter {
    fn paint(&mut self, snapshot: &Snapshot);
}

/// When the dialog may be drawn again.
#[derive(Debug, Clone, Copy)]
struct Throttle {
    started: Instant,
    last: Option<Instant>,
}

impl Throttle {
    const fn new(started: Instant) -> Self {
        Self {
            started,
            last: None,
        }
    }

    /// Whether a frame drawn at `now` would be `interval` after the last one
    /// — or, for the first, `interval` after the operation started.
    fn due(&self, now: Instant, interval: Duration) -> bool {
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
        let (phase, files) = if operation.scans_first() {
            (Phase::Scanning, 0)
        } else {
            (
                Phase::Working,
                u64::try_from(sources.len()).unwrap_or(u64::MAX),
            )
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

/// Start showing `operation` on `sources`. A copy or a delete measures them
/// first, reporting as it goes, so the dialog can say how far along it is.
pub fn begin(operation: Operation, sources: &[PathBuf]) -> Run {
    let started = with_screen(|screen| {
        if screen.shown.is_some() {
            return false;
        }
        screen.shown = Some(Shown::new(operation, sources, Instant::now()));
        true
    })
    .unwrap_or(false);
    if started && operation.scans_first() {
        for source in sources {
            measure(source);
        }
        update(|shown| shown.phase = Phase::Working);
    }
    Run { owns: started }
}

/// The operation is at `path` now.
pub fn entering(path: &Path) {
    update(|shown| shown.current = path.to_path_buf());
}

/// A copy wrote `bytes` more.
pub fn copied(bytes: u64) {
    update(|shown| {
        if shown.operation != Operation::Delete {
            shown.done.bytes = shown.done.bytes.saturating_add(bytes);
        }
    });
}

/// A copy finished one file or link.
pub fn file_copied() {
    update(|shown| {
        if shown.operation == Operation::Copy {
            shown.done.files = shown.done.files.saturating_add(1);
        }
    });
}

/// `path` was removed: one file less for a delete, and a move's source
/// going for a move across filesystems.
pub fn removed(path: &Path) {
    update(|shown| {
        shown.current = path.to_path_buf();
        if shown.operation == Operation::Delete {
            shown.done.files = shown.done.files.saturating_add(1);
        }
    });
}

/// A move has to copy `source` across filesystems: its bytes join what the
/// dialog counts, so a big folder moved to another disk fills the bar as
/// it is copied.
pub fn copying_across(source: &Path) {
    if !is_shown() {
        return;
    }
    measure(source);
}

/// One of the selected entries is done. A move counts them, since a rename
/// moves a whole folder in one step; a copy and a delete count every file.
pub fn source_done() {
    update(|shown| {
        if shown.operation == Operation::Move {
            shown.done.files = shown.done.files.saturating_add(1);
        }
    });
}

/// Draw `snapshot`, which another process sent: the administrator helper
/// measures and throttles its own operation, and this one only shows it.
pub fn relay(snapshot: &Snapshot) {
    with_screen(|screen| {
        screen.painter.paint(snapshot);
        screen.painted = true;
    });
}

/// Whether an operation is being shown on this thread.
#[must_use]
pub fn is_shown() -> bool {
    with_screen(|screen| screen.shown.is_some()).unwrap_or(false)
}

/// Add what is under `root` to the operation's total, without following a
/// link: see [`found`].
fn measure(root: &Path) {
    let mut folders = vec![root.to_path_buf()];
    let Ok(metadata) = fs::symlink_metadata(root) else {
        return;
    };
    if !metadata.is_dir() {
        found(root, &metadata);
        return;
    }
    while let Some(folder) = folders.pop() {
        entering(&folder);
        // Unreadable folders are the operation's to report; the count only
        // comes out short.
        let Ok(entries) = fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(metadata) = fs::symlink_metadata(entry.path()) else {
                continue;
            };
            if metadata.is_dir() {
                folders.push(entry.path());
            } else {
                found(&entry.path(), &metadata);
            }
        }
    }
}

/// Count one file or link found by [`measure`]: as a file to do for a copy
/// or a delete, whose bar counts them, and by the bytes a copy or a move
/// across filesystems will read. A move's files are already counted by the
/// entries selected; see [`source_done`].
fn found(path: &Path, metadata: &fs::Metadata) {
    let size = if metadata.is_file() {
        metadata.len()
    } else {
        0
    };
    update(|shown| {
        shown.current = path.to_path_buf();
        if shown.operation != Operation::Move {
            shown.total.files = shown.total.files.saturating_add(1);
        }
        if shown.operation != Operation::Delete {
            shown.total.bytes = shown.total.bytes.saturating_add(size);
        }
    });
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

#[cfg(test)]
mod tests;
