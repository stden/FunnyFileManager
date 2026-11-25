//! Purpose: The painter that draws the progress dialog of a long copy, move
//! or delete, or the top-border banner of a long directory read, straight onto
//! the terminal over the last frame (see `progress`).

use std::io::Write;

use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::progress::{DirectoryRead, Painter, Snapshot};

/// Writes the progress cells to `out`, a terminal of the size `size` reports.
pub struct ProgressScreen<W: Write> {
    backend: CrosstermBackend<W>,
    size: fn() -> (u16, u16),
}

impl ProgressScreen<std::io::Stdout> {
    /// The painter for the session's own terminal.
    #[must_use]
    pub fn stdout() -> Self {
        Self::new(std::io::stdout(), || {
            crossterm::terminal::size().unwrap_or((80, 25))
        })
    }
}

impl<W: Write> ProgressScreen<W> {
    pub fn new(out: W, size: fn() -> (u16, u16)) -> Self {
        Self {
            backend: CrosstermBackend::new(out),
            size,
        }
    }

    fn draw_buffer(&mut self, buffer: &Buffer) {
        let cells = buffer.content.iter().enumerate().map(|(index, cell)| {
            let (x, y) = buffer.pos_of(index);
            (x, y, cell)
        });
        // Progress that cannot be drawn must not stop the operation: the next
        // frame after it redraws the whole screen anyway.
        drop(self.backend.draw(cells));
        drop(Backend::flush(&mut self.backend));
    }
}

impl<W: Write> Painter for ProgressScreen<W> {
    fn paint(&mut self, snapshot: &Snapshot) {
        if super::headless::installed() {
            return;
        }
        let (width, height) = (self.size)();
        let dialog = crate::ui::progress_dialog(Rect::new(0, 0, width, height), snapshot);
        self.draw_buffer(&dialog);
    }

    fn paint_directory_read(&mut self, read: &DirectoryRead) {
        if super::headless::installed() {
            return;
        }
        let (width, height) = (self.size)();
        let banner = crate::ui::reading_banner(Rect::new(0, 0, width, height), read);
        self.draw_buffer(&banner);
    }
}

#[cfg(test)]
mod tests;
