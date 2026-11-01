//! Purpose: The painter that draws the progress dialog of a long copy, move
//! or delete straight onto the terminal, over the last frame, while the
//! operation still holds the application state (see `progress`).

use std::io::Write;

use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::layout::Rect;

use crate::progress::{Painter, Snapshot};

/// Writes the dialog's cells to `out`, a terminal of the size `size` reports.
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
}

impl<W: Write> Painter for ProgressScreen<W> {
    fn paint(&mut self, snapshot: &Snapshot) {
        if super::headless::installed() {
            return;
        }
        let (width, height) = (self.size)();
        let dialog = crate::ui::progress_dialog(Rect::new(0, 0, width, height), snapshot);
        let cells = dialog.content.iter().enumerate().map(|(index, cell)| {
            let (x, y) = dialog.pos_of(index);
            (x, y, cell)
        });
        // A dialog that cannot be drawn must not stop the operation: the
        // next frame after it redraws the whole screen anyway.
        drop(self.backend.draw(cells));
        drop(Backend::flush(&mut self.backend));
    }
}

#[cfg(test)]
mod tests;
