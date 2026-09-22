//! Nonblocking editor input batching. Never wait to guess whether typing is a paste.

use std::io;

use crossterm::event::{Event, KeyEventKind};

use crate::{
    action::editor::{resolve, EditorCommand},
    models::App,
};

/// Bound queue draining so a long stream still yields frames and background work.
const MAX_BATCH_EVENTS: usize = 16_384;

pub trait EventSource {
    fn read(&mut self) -> io::Result<Option<Event>>;

    fn try_read(&mut self) -> io::Result<Option<Event>> {
        Ok(None)
    }
}

impl<F: FnMut() -> io::Result<Option<Event>>> EventSource for F {
    fn read(&mut self) -> io::Result<Option<Event>> {
        self()
    }
}

impl EventSource for &mut crate::recording::Input {
    fn read(&mut self) -> io::Result<Option<Event>> {
        crate::recording::Input::read(self)
    }

    fn try_read(&mut self) -> io::Result<Option<Event>> {
        self.read_pending()
    }
}

pub struct BufferedInput<S> {
    source: S,
    pending: Option<io::Result<Event>>,
}

impl<S: EventSource> BufferedInput<S> {
    pub(crate) fn new(source: S) -> Self {
        Self {
            source,
            pending: None,
        }
    }

    pub(crate) fn read(&mut self, app: &App) -> io::Result<Option<Event>> {
        let event = self
            .pending
            .take()
            .map_or_else(|| self.source.read(), |next| next.map(Some))?;
        if app
            .get_editor()
            .is_some_and(crate::models::Editor::accepts_text)
        {
            Ok(event.map(|event| self.coalesce(event)))
        } else {
            Ok(event)
        }
    }

    fn coalesce(&mut self, event: Event) -> Event {
        let Some(first) = text_character(&event) else {
            return event;
        };
        let mut text = first.to_string();
        for _ in 1..MAX_BATCH_EVENTS {
            if !self.extend(&mut text) {
                break;
            }
        }
        if text.len() > first.len_utf8() {
            Event::Paste(text)
        } else {
            event
        }
    }

    fn extend(&mut self, text: &mut String) -> bool {
        match self.source.try_read() {
            Ok(Some(next)) => self.append(next, text),
            other => {
                self.pending = other.transpose();
                false
            }
        }
    }

    fn append(&mut self, event: Event, text: &mut String) -> bool {
        if let Some(ch) = text_character(&event) {
            text.push(ch);
            true
        } else if ignored_key(&event) {
            true
        } else {
            self.pending = Some(Ok(event));
            false
        }
    }
}

fn text_character(event: &Event) -> Option<char> {
    let Event::Key(key) = event else {
        return None;
    };
    if key.kind != KeyEventKind::Press {
        return None;
    }
    match resolve(*key) {
        EditorCommand::Insert(ch) => Some(ch),
        EditorCommand::Newline => Some('\n'),
        _ => None,
    }
}

fn ignored_key(event: &Event) -> bool {
    matches!(event, Event::Key(key) if key.kind != KeyEventKind::Press)
}

#[cfg(test)]
mod tests;
