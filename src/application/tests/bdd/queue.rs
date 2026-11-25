//! A real nonblocking event source: queued bursts never wait for more input.

use super::super::idle_render::backend::CountingBackend;
use crate::application::input::EventSource;
use crossterm::event::Event;
use ratatui::layout::Size;
use std::{collections::VecDeque, io};

pub(super) struct Queue {
    pub events: VecDeque<Event>,
    pub screen: CountingBackend,
}

impl EventSource for Queue {
    fn read(&mut self) -> io::Result<Option<Event>> {
        let event = self
            .events
            .pop_front()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Interrupted, "BDD input complete"))?;
        if let Event::Resize(width, height) = event {
            self.screen.resize(Size::new(width, height));
        }
        Ok(Some(event))
    }

    fn try_read(&mut self) -> io::Result<Option<Event>> {
        // A resize changes the simulated host only when it is dispatched,
        // after the frame showing the preceding queued movements.
        if matches!(self.events.front(), Some(Event::Resize(..))) {
            return Ok(None);
        }
        Ok(self.events.pop_front())
    }
}
