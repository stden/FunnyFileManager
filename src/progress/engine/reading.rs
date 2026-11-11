//! Purpose: Throttled top-border progress tracking for long directory reads.

use std::time::{Duration, Instant};

use super::{with_screen, Throttle};

/// Where a panel's top border sits on the terminal screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PanelHeader {
    pub x: u16,
    pub y: u16,
    pub width: u16,
}

/// One state of a long directory read, drawn in the panel's top border.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DirectoryRead {
    pub files: u64,
    pub header: Option<PanelHeader>,
}

struct ActiveRead {
    files: u64,
    header: Option<PanelHeader>,
    throttle: Throttle,
    interval: Duration,
    visible: bool,
}

/// Tracks one directory read until it finishes or is interrupted.
#[must_use = "the directory read stops being shown as soon as this is dropped"]
pub struct DirectoryReadRun {
    active: Option<ActiveRead>,
}

impl DirectoryReadRun {
    /// Record one more entry read from the directory; returns `true` once
    /// the read has run long enough to show its top-border banner.
    pub fn step(&mut self) -> bool {
        let Some(active) = self.active.as_mut() else {
            return false;
        };
        active.files = active.files.saturating_add(1);
        let now = Instant::now();
        if active.throttle.due(now, active.interval) {
            active.throttle.last = Some(now);
            active.visible = true;
            let read = DirectoryRead {
                files: active.files,
                header: active.header,
            };
            with_screen(|screen| {
                screen.painter.paint_directory_read(&read);
                screen.painted = true;
            });
        }
        active.visible
    }
}

impl Drop for DirectoryReadRun {
    fn drop(&mut self) {
        if self.active.is_some() {
            with_screen(|screen| screen.reading = false);
        }
    }
}

/// Start tracking a directory read for the panel whose top border is at `header`.
pub fn begin_directory_read(header: Option<PanelHeader>) -> DirectoryReadRun {
    let interval = with_screen(|screen| {
        if screen.shown.is_some() || screen.reading {
            return None;
        }
        screen.reading = true;
        Some(screen.interval)
    })
    .flatten();
    let active = interval.map(|interval| ActiveRead {
        files: 0,
        header,
        throttle: Throttle::new(Instant::now()),
        interval,
        visible: false,
    });
    DirectoryReadRun { active }
}
