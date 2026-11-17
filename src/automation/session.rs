//! Purpose: Serialize application jobs while keeping state, cancellation and private terminal input available.
use super::{
    catalog,
    options::{invalid, Options},
    protocol,
};
use crate::terminal::headless::{ExternalState, HeadlessHost};
use ratatui::layout::Size;
use serde_json::{json, Value};
use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender},
        Arc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

mod command;
mod external;
mod progress;
mod published;
mod worker;
use command::{Command, Operation};
use published::Shared;

pub(super) struct Session {
    sender: SyncSender<Command>,
    shared: Shared,
    host: HeadlessHost,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Session {
    pub(super) fn start(options: Options) -> io::Result<Self> {
        let (sender, receiver) = mpsc::sync_channel(1);
        let (ready, started) = mpsc::sync_channel(1);
        let shared = Shared::default();
        let host = HeadlessHost::new();
        host.resize(options.size.width, options.size.height)?;
        let stop = Arc::new(AtomicBool::new(false));
        let (state, private_host, stopped) = (shared.clone(), host.clone(), Arc::clone(&stop));
        let worker = std::thread::Builder::new()
            .name("ffm-automation".to_owned())
            .spawn(move || {
                worker::run(&options, &receiver, &state, &private_host, &stopped, &ready);
            })?;
        let session = Self {
            sender,
            shared,
            host,
            stop,
            worker: Some(worker),
        };
        started
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_error| io::Error::other("FFM session startup timed out"))?
            .map_err(io::Error::other)?;
        Ok(session)
    }

    pub(super) fn state(&self) -> io::Result<Value> {
        let mut state = self.shared.snapshot()?;
        if let Some(object) = state.as_object_mut() {
            object.insert(
                "external_process".to_owned(),
                external::snapshot(&self.host),
            );
        }
        Ok(state)
    }

    pub(super) fn action(&self, context: &str, action: &str) -> io::Result<Value> {
        let invocation =
            catalog::resolve(context, action).map_err(|error| invalid(&error.to_string()))?;
        self.enqueue(Operation::Action(invocation))
    }

    pub(super) fn input(
        &self,
        keys: Vec<String>,
        text: Option<String>,
        mouse: Option<protocol::Mouse>,
    ) -> io::Result<Value> {
        let events = protocol::input::events(keys, text, mouse)?;
        if self.host.snapshot().state == ExternalState::Running {
            external::input(&self.host, events)?;
            return self.response();
        }
        if self.shared.is_busy()? && events.iter().any(crate::terminal::is_cancel_event) {
            return self.cancel();
        }
        self.enqueue(Operation::Input(events))
    }

    pub(super) fn resize(&self, width: u16, height: u16) -> io::Result<Value> {
        if !(20..=400).contains(&width) || !(8..=200).contains(&height) {
            return Err(invalid(
                "automation dimensions are outside the supported range",
            ));
        }
        self.host.resize(width, height)?;
        if self.host.snapshot().state == ExternalState::Running {
            return self.response();
        }
        self.enqueue(Operation::Resize(Size::new(width, height)))
    }

    pub(super) fn wait(&self, timeout: u64) -> io::Result<Value> {
        if timeout > 10_000 {
            return Err(invalid("wait timeout exceeds 10000 ms"));
        }
        let deadline = Instant::now() + Duration::from_millis(timeout);
        loop {
            let state = self.state()?;
            let busy = state.get("busy").and_then(Value::as_bool) == Some(true);
            let pending = state.get("background_pending").and_then(Value::as_bool) == Some(true);
            if (!busy && !pending) || Instant::now() >= deadline {
                return Ok(json!({"state": state}));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    pub(super) fn cancel(&self) -> io::Result<Value> {
        self.host.cancel();
        self.response()
    }

    pub(super) fn response(&self) -> io::Result<Value> {
        Ok(json!({"state": self.state()?}))
    }

    pub(super) fn close(&mut self) -> io::Result<Value> {
        self.stop.store(true, Ordering::Release);
        self.host.cancel();
        let deadline = Instant::now() + Duration::from_millis(500);
        while self
            .worker
            .as_ref()
            .is_some_and(|worker| !worker.is_finished())
            && Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(5));
        }
        if self.worker.as_ref().is_some_and(JoinHandle::is_finished) {
            if let Some(worker) = self.worker.take() {
                drop(worker.join());
            }
        }
        self.shared.stop(None)?;
        self.response()
    }

    fn enqueue(&self, operation: Operation) -> io::Result<Value> {
        let id = self.shared.reserve()?;
        if self.sender.try_send(Command { id, operation }).is_err() {
            self.shared
                .complete(id, Some("session worker is unavailable".to_owned()))?;
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "session worker is unavailable",
            ));
        }
        Ok(json!({"job_id": id, "state": self.state()?}))
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.host.cancel();
    }
}

#[cfg(test)]
mod tests;
