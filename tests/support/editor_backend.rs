//! Share the real test backend with an event script for frame inspection and resize.
//!
//! The application still owns `Terminal` for the whole run. The script only
//! accesses its backend between frames, without aliasing the terminal itself.

use std::{cell::RefCell, io, io::Write, rc::Rc};

use funny_file_manager::WritableTestBackend;
use ratatui::{
    backend::{Backend, ClearType, WindowSize},
    buffer::{Buffer, Cell},
    layout::{Position, Size},
};

#[derive(Clone)]
pub struct SharedBackend(Rc<RefCell<WritableTestBackend>>);

impl SharedBackend {
    pub fn new(width: u16, height: u16) -> Self {
        Self(Rc::new(RefCell::new(WritableTestBackend::new(
            width, height,
        ))))
    }

    pub fn buffer(&self) -> Buffer {
        self.0.borrow().buffer().clone()
    }

    pub fn resize(&self, size: Size) {
        *self.0.borrow_mut() = WritableTestBackend::new(size.width, size.height);
    }
}

impl Write for SharedBackend {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        Write::flush(&mut *self.0.borrow_mut())
    }
}

impl Backend for SharedBackend {
    type Error = io::Error;

    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        self.0.borrow_mut().draw(content)
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.0.borrow_mut().hide_cursor()
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.0.borrow_mut().show_cursor()
    }

    fn get_cursor_position(&mut self) -> io::Result<Position> {
        self.0.borrow_mut().get_cursor_position()
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> io::Result<()> {
        self.0.borrow_mut().set_cursor_position(position)
    }

    fn clear(&mut self) -> io::Result<()> {
        self.0.borrow_mut().clear()
    }

    fn clear_region(&mut self, clear_type: ClearType) -> io::Result<()> {
        self.0.borrow_mut().clear_region(clear_type)
    }

    fn size(&self) -> io::Result<Size> {
        self.0.borrow().size()
    }

    fn flush(&mut self) -> io::Result<()> {
        Backend::flush(&mut *self.0.borrow_mut())
    }

    fn window_size(&mut self) -> io::Result<WindowSize> {
        self.0.borrow_mut().window_size()
    }
}
