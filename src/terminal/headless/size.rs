//! Purpose: Valid private-terminal dimensions shared by the viewport and PTY.

use portable_pty::PtySize;
use ratatui::layout::Size;
use std::io;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct PrivateSize(Size);

impl PrivateSize {
    pub fn new(width: u16, height: u16) -> io::Result<Self> {
        // ConPTY represents dimensions with signed 16-bit COORD members.
        if !(1..=32_767).contains(&width) || !(1..=32_767).contains(&height) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid private terminal size",
            ));
        }
        Ok(Self(Size::new(width, height)))
    }

    pub fn screen(self) -> Size {
        self.0
    }

    pub fn pty(self) -> PtySize {
        PtySize {
            cols: self.0.width,
            rows: self.0.height,
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}

impl Default for PrivateSize {
    fn default() -> Self {
        Self(Size::new(120, 40))
    }
}
