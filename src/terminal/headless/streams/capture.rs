//! Purpose: Record one private terminal's output and queue its protocol replies.

use super::super::{
    responses::{Queries, Reply},
    state::{Captured, Generation},
    HeadlessHost,
};
use std::{
    io,
    sync::{
        mpsc::{SyncSender, TrySendError},
        Arc, Mutex,
    },
};

pub(super) struct Capture {
    host: HeadlessHost,
    generation: Generation,
    captured: Arc<Mutex<Captured>>,
    replies: SyncSender<Reply>,
    queries: Queries,
}

impl Capture {
    pub fn new(
        host: HeadlessHost,
        generation: Generation,
        captured: Arc<Mutex<Captured>>,
        replies: SyncSender<Reply>,
    ) -> Self {
        Self {
            host,
            generation,
            captured,
            replies,
            queries: Queries::default(),
        }
    }

    #[cfg(unix)]
    pub fn active(&self) -> bool {
        self.host.active(self.generation) && !self.host.is_cancelled()
    }

    pub fn record(&mut self, buffer: &[u8], count: usize) -> io::Result<()> {
        let bytes = buffer
            .get(..count)
            .ok_or_else(|| io::Error::other("Invalid terminal read count"))?;
        respond(&self.replies, self.queries.feed(bytes));
        self.captured
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .append(bytes);
        self.host.append(self.generation, bytes);
        Ok(())
    }
}

fn respond(replies: &SyncSender<Reply>, count: usize) {
    for _ in 0..count {
        match replies.try_send(Reply::CursorPosition) {
            Ok(()) => {}
            // Coalesce repeated queries while terminal replies are pending.
            Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => break,
        }
    }
}

#[cfg(test)]
mod tests;
