//! Nonblocking editor input batching. Never wait to guess whether typing is a paste.

use std::io;

use crossterm::event::{Event, KeyEventKind};

use crate::{
    action::editor::{resolve, EditorCommand},
    action::Bindings,
    models::App,
};

/// Bound queue draining so a long stream still yields frames and background work.
const MAX_BATCH_EVENTS: usize = 16_384;

/// The editor's keys in force, which say what is text and what is a command.
type EditorKeys = Bindings<EditorCommand>;

pub trait EventSource {
    fn read(&mut self) -> io::Result<Option<Event>>;

    fn try_read(&mut self) -> io::Result<Option<Event>> {
        Ok(None)
    }

    /// Throw away the keys typed at a screen that did not answer, and say
    /// whether one of them was the cancel gesture.
    ///
    /// Only a source that models a live keyboard has such keys, and only it
    /// knows how to drop them (see `recording::Input`). A scripted source —
    /// a replay, a test's queue — keeps everything: its events are the
    /// session, not what someone typed into a freeze, and how long an event
    /// took on this machine says nothing about them. The default therefore
    /// discards nothing; a source with a real queue overrides it.
    fn discard_queued(&mut self) -> io::Result<bool> {
        Ok(false)
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

    fn discard_queued(&mut self) -> io::Result<bool> {
        crate::recording::Input::discard_queued(self)
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
            Ok(event.map(|event| self.coalesce(&app.keymap.editor, event)))
        } else {
            Ok(event)
        }
    }

    /// Throw away what was typed while the loop was busy; see
    /// [`EventSource::discard_queued`]. An event already read ahead stays: it
    /// was typed before the busy event began, as part of the same burst.
    pub(crate) fn discard_queued(&mut self) -> io::Result<bool> {
        self.source.discard_queued()
    }

    /// Take queued navigation without waiting. Keep the next command or error
    /// for the normal reader so the current selection is presented first.
    pub(super) fn next_navigation(&mut self, app: &App) -> Option<Event> {
        let next = self
            .pending
            .take()
            .map_or_else(|| self.source.try_read().transpose(), Some);
        match next {
            Some(Ok(event)) if super::navigation_batch::accepts(app, &event) => Some(event),
            other => {
                self.pending = other;
                None
            }
        }
    }

    fn coalesce(&mut self, keys: &EditorKeys, event: Event) -> Event {
        let Some(first) = text_character(keys, &event) else {
            return event;
        };
        let mut text = first.to_string();
        for _ in 1..MAX_BATCH_EVENTS {
            if !self.extend(keys, &mut text) {
                break;
            }
        }
        if text.len() > first.len_utf8() {
            Event::Paste(text)
        } else {
            event
        }
    }

    fn extend(&mut self, keys: &EditorKeys, text: &mut String) -> bool {
        match self.source.try_read() {
            Ok(Some(next)) => self.append(keys, next, text),
            other => {
                self.pending = other.transpose();
                false
            }
        }
    }

    fn append(&mut self, keys: &EditorKeys, event: Event, text: &mut String) -> bool {
        if let Some(ch) = text_character(keys, &event) {
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

/// The character a key puts into the text, by the editor's keys in force: a
/// typed character, or a line break for whatever key starts a line.
fn text_character(keys: &EditorKeys, event: &Event) -> Option<char> {
    let Event::Key(key) = event else {
        return None;
    };
    if key.kind != KeyEventKind::Press {
        return None;
    }
    match resolve(keys, *key) {
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
