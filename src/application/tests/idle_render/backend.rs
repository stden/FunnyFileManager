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
    writes: Vec<u8>,
    draw_failure: Option<io::ErrorKind>,
}

#[derive(Clone)]
pub(in crate::application::tests) struct CountingBackend(Rc<RefCell<State>>);

impl CountingBackend {
    pub(in crate::application::tests) fn new() -> Self {
        Self(Rc::new(RefCell::new(State {
            backend: WritableTestBackend::new(80, 20),
            draws: 0,
            writes: Vec::new(),
            draw_failure: None,
        })))
    }

    pub(in crate::application::tests) fn draws(&self) -> usize {
        self.0.borrow().draws
    }

    pub(in crate::application::tests) fn writes(&self) -> Vec<u8> {
        self.0.borrow().writes.clone()
    }

    pub(in crate::application::tests) fn fail_next_draw(&self) {
        self.0.borrow_mut().draw_failure = Some(io::ErrorKind::BrokenPipe);
    }

    pub(in crate::application::tests) fn resize(&self, size: Size) {
        self.0.borrow_mut().backend = WritableTestBackend::new(size.width, size.height);
    }

    pub(in crate::application::tests) fn text(&self) -> String {
        self.0
            .borrow()
            .backend
            .buffer()
            .content
            .iter()
            .map(Cell::symbol)
            .collect()
    }

    pub(in crate::application::tests) fn rows(&self) -> Vec<String> {
        let state = self.0.borrow();
        let buffer = state.backend.buffer();
        buffer
            .content
            .chunks(usize::from(buffer.area.width).max(1))
            .map(|row| row.iter().map(Cell::symbol).collect())
            .collect()
    }
}

impl Write for CountingBackend {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().writes.extend_from_slice(bytes);
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
        if let Some(kind) = state.draw_failure.take() {
            return Err(io::Error::new(kind, "draw failed"));
        }
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

    fn window_size(&mut self) -> io::Result<WindowSize> {
        self.0.borrow_mut().backend.window_size()
    }

    fn flush(&mut self) -> io::Result<()> {
        Backend::flush(&mut self.0.borrow_mut().backend)
    }
}
