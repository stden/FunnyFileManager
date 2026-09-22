//! Count real backend draws while permitting inspection between input events.

use std::{cell::RefCell, io, io::Write, rc::Rc};

use ratatui::{
    backend::{Backend, ClearType, WindowSize},
    buffer::Cell,
    layout::{Position, Size},
};

use crate::WritableTestBackend;

struct State {
    backend: WritableTestBackend,
    draws: usize,
}

#[derive(Clone)]
pub(super) struct CountingBackend(Rc<RefCell<State>>);

impl CountingBackend {
    pub(super) fn new() -> Self {
        Self(Rc::new(RefCell::new(State {
            backend: WritableTestBackend::new(80, 20),
            draws: 0,
        })))
    }

    pub(super) fn draws(&self) -> usize {
        self.0.borrow().draws
    }

    pub(super) fn resize(&self, size: Size) {
        self.0.borrow_mut().backend = WritableTestBackend::new(size.width, size.height);
    }

    pub(super) fn text(&self) -> String {
        self.0
            .borrow()
            .backend
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect()
    }
}

impl Write for CountingBackend {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().backend.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        Write::flush(&mut self.0.borrow_mut().backend)
    }
}

impl Backend for CountingBackend {
    type Error = io::Error;

    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        let mut state = self.0.borrow_mut();
        state.backend.draw(content)?;
        state.draws += 1;
        Ok(())
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.0.borrow_mut().backend.hide_cursor()
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.0.borrow_mut().backend.show_cursor()
    }

    fn get_cursor_position(&mut self) -> io::Result<Position> {
        self.0.borrow_mut().backend.get_cursor_position()
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> io::Result<()> {
        self.0.borrow_mut().backend.set_cursor_position(position)
    }

    fn clear(&mut self) -> io::Result<()> {
        self.0.borrow_mut().backend.clear()
    }

    fn clear_region(&mut self, clear_type: ClearType) -> io::Result<()> {
        self.0.borrow_mut().backend.clear_region(clear_type)
    }

    fn size(&self) -> io::Result<Size> {
        self.0.borrow().backend.size()
    }

    fn flush(&mut self) -> io::Result<()> {
        Backend::flush(&mut self.0.borrow_mut().backend)
    }

    fn window_size(&mut self) -> io::Result<WindowSize> {
        self.0.borrow_mut().backend.window_size()
    }
}
