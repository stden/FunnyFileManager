//! Writable ratatui [`TestBackend`] used by unit and integration tests.

use ratatui::backend::{Backend, ClearType, TestBackend, WindowSize};
use ratatui::layout::{Position, Size};
use std::convert::Infallible;
use std::io::{self, Write};

/// `TestBackend`'s associated error is [`Infallible`]; this lifts it into the
/// [`io::Error`] that `WritableTestBackend` exposes (the conversion never runs).
#[inline]
const fn never(e: Infallible) -> io::Error {
    match e {}
}

pub struct WritableTestBackend {
    pub(crate) inner: TestBackend,
}

impl WritableTestBackend {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            inner: TestBackend::new(width, height),
        }
    }

    /// The cells the backend actually holds — the frame a user would be
    /// looking at.
    ///
    /// Not `Terminal::current_buffer_mut`: ratatui double-buffers, so that one
    /// is the *next* frame and is blank. Reading the wrong one makes every
    /// screen assertion pass against emptiness, which is the failure mode this
    /// accessor exists to remove.
    #[must_use]
    pub const fn buffer(&self) -> &ratatui::buffer::Buffer {
        self.inner.buffer()
    }
}

impl Write for WritableTestBackend {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Backend for WritableTestBackend {
    type Error = io::Error;

    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a ratatui::buffer::Cell)>,
    {
        self.inner.draw(content).map_err(never)
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.inner.hide_cursor().map_err(never)
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.inner.show_cursor().map_err(never)
    }

    fn get_cursor_position(&mut self) -> io::Result<Position> {
        self.inner.get_cursor_position().map_err(never)
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> io::Result<()> {
        self.inner.set_cursor_position(position).map_err(never)
    }

    fn clear(&mut self) -> io::Result<()> {
        self.inner.clear().map_err(never)
    }

    fn clear_region(&mut self, clear_type: ClearType) -> io::Result<()> {
        self.inner.clear_region(clear_type).map_err(never)
    }

    fn size(&self) -> io::Result<Size> {
        self.inner.size().map_err(never)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush().map_err(never)
    }

    fn window_size(&mut self) -> io::Result<WindowSize> {
        let size = self.size()?;
        Ok(WindowSize {
            columns_rows: size,
            pixels: Size {
                width: size.width * 8,
                height: size.height * 16,
            },
        })
    }
}

#[cfg(test)]
mod tests;
