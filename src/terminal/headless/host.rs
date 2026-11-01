//! Purpose: Thread-safe session control without inheriting MCP's input/output.

use super::{
    state::{Captured, ExternalState, Generation, Session, SessionState},
    HeadlessScope, HeadlessSnapshot, ACTIVE,
};
use crossterm::event::KeyEvent;
use std::{
    io,
    marker::PhantomData,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{SyncSender, TrySendError},
        Arc, Mutex, MutexGuard,
    },
};

#[derive(Clone, Default)]
pub struct HeadlessHost(Arc<Shared>);

#[derive(Default)]
struct Shared {
    session: Mutex<Session>,
    cancelled: AtomicBool,
}

impl HeadlessHost {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enter(&self) -> HeadlessScope {
        HeadlessScope {
            previous: ACTIVE.with(|active| active.replace(Some(self.clone()))),
            thread: PhantomData,
        }
    }

    pub fn snapshot(&self) -> HeadlessSnapshot {
        let (state, bytes, dropped_bytes) = {
            let session = self.session();
            (
                session.state.public(),
                session.output.bytes(),
                session.output.dropped,
            )
        };
        HeadlessSnapshot {
            state,
            output: String::from_utf8_lossy(&bytes).into_owned(),
            dropped_bytes,
        }
    }

    pub fn input(&self, input: &str) -> io::Result<usize> {
        if input.len() > 8192 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Private input exceeds 8192 bytes",
            ));
        }
        let sender = {
            let session = self.session();
            match &session.state {
                SessionState::Running(sender) => sender.clone(),
                SessionState::Ready(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::NotConnected,
                        "No external program is running",
                    ))
                }
            }
        };
        sender
            .try_send(input.as_bytes().to_vec())
            .map_err(|error| input_error(&error))?;
        Ok(input.len())
    }

    pub fn resize(&self, width: u16, height: u16) -> io::Result<()> {
        self.session().size = super::size::PrivateSize::new(width, height)?;
        Ok(())
    }

    pub fn size(&self) -> ratatui::layout::Size {
        self.session().size.screen()
    }

    pub(super) fn pty_size(&self) -> portable_pty::PtySize {
        self.session().size.pty()
    }

    pub fn key(&self, key: KeyEvent) -> io::Result<usize> {
        let input = super::keys::encode(key)?;
        if input.is_empty() {
            return Ok(0);
        }
        self.input(&input)
    }

    pub fn cancel(&self) {
        self.0.cancelled.store(true, Ordering::Release);
    }

    #[allow(
        clippy::significant_drop_tightening,
        reason = "drop order is intentional in this mutex guard pattern"
    )]
    pub fn reset(&self) -> io::Result<()> {
        let mut session = self.session();
        if matches!(session.state, SessionState::Running(_)) {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "An external program is running",
            ));
        }
        session.generation.0 = session.generation.0.wrapping_add(1);
        session.output = Captured::default();
        session.state = SessionState::Ready(ExternalState::Idle);
        self.0.cancelled.store(false, Ordering::Release);
        Ok(())
    }

    pub(super) fn is_cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::Acquire)
    }

    pub(super) fn begin(&self, input: SyncSender<Vec<u8>>) -> io::Result<Generation> {
        let mut session = self.session();
        if matches!(session.state, SessionState::Running(_)) {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "An external program is running",
            ));
        }
        if self.is_cancelled() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Command cancelled",
            ));
        }
        session.generation.0 = session.generation.0.wrapping_add(1);
        session.state = SessionState::Running(input);
        Ok(session.generation)
    }

    pub(super) fn append(&self, generation: Generation, bytes: &[u8]) {
        let mut session = self.session();
        if session.generation == generation {
            session.output.append(bytes);
        }
    }

    pub(super) fn active(&self, generation: Generation) -> bool {
        let session = self.session();
        session.generation == generation && matches!(session.state, SessionState::Running(_))
    }

    pub(super) fn finish(&self, generation: Generation, state: ExternalState) {
        let mut session = self.session();
        if session.generation == generation {
            session.state = SessionState::Ready(state);
        }
    }

    fn session(&self) -> MutexGuard<'_, Session> {
        // A snapshot remains readable even if an action panics while holding it.
        self.0
            .session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

fn input_error(error: &TrySendError<Vec<u8>>) -> io::Error {
    match error {
        TrySendError::Full(_) => {
            io::Error::new(io::ErrorKind::WouldBlock, "Private input queue is full")
        }
        TrySendError::Disconnected(_) => {
            io::Error::new(io::ErrorKind::BrokenPipe, "External input closed")
        }
    }
}
