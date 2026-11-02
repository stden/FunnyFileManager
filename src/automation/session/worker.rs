//! Purpose: Own App on one engine thread while transport requests stay responsive.
use super::{
    command::{Command, Operation},
    published::Shared,
};
use crate::{
    application::headless::Runtime,
    automation::{options::Options, snapshot},
    terminal::headless::HeadlessHost,
};
use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, RecvTimeoutError, SyncSender},
    },
    time::Duration,
};

pub(super) fn run(
    options: &Options,
    receiver: &Receiver<Command>,
    shared: &Shared,
    host: &HeadlessHost,
    stop: &AtomicBool,
    started: &SyncSender<Result<(), String>>,
) {
    let _host = host.enter();
    let _progress = crate::progress::install(Box::new(super::progress::Reporter(shared.clone())));
    let Some(mut runtime) = initialize(options, shared, started) else {
        return;
    };
    drive(&mut runtime, receiver, shared, host, stop);
}

fn initialize(
    options: &Options,
    shared: &Shared,
    started: &SyncSender<Result<(), String>>,
) -> Option<Runtime> {
    let runtime = Runtime::new(&options.root, options.size, options.profile);
    let Ok(runtime) = runtime else {
        drop(started.send(Err("cannot initialize the FFM session".to_owned())));
        return None;
    };
    let startup = publish(&runtime, shared).map_err(|error| error.to_string());
    if startup.is_err() || started.send(startup).is_err() {
        return None;
    }
    Some(runtime)
}

fn drive(
    runtime: &mut Runtime,
    receiver: &Receiver<Command>,
    shared: &Shared,
    host: &HeadlessHost,
    stop: &AtomicBool,
) {
    while !stop.load(Ordering::Acquire) && runtime.app.lifecycle.is_running() {
        let was_pending = snapshot::is_pending(&runtime.app);
        match receiver.recv_timeout(Duration::from_millis(25)) {
            Ok(command) => execute(runtime, shared, host, command),
            Err(RecvTimeoutError::Timeout) => {
                if !refresh_idle(runtime, shared, was_pending) {
                    drop(shared.stop(Some("application refresh failed".to_owned())));
                    return;
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    drop(publish(runtime, shared));
    drop(shared.stop(None));
}

fn refresh_idle(runtime: &mut Runtime, shared: &Shared, was_pending: bool) -> bool {
    let Ok(redrew) = runtime.pump() else {
        return false;
    };
    if !redrew && snapshot::is_pending(&runtime.app) == was_pending {
        return true;
    }
    publish(runtime, shared).is_ok()
}

fn execute(runtime: &mut Runtime, shared: &Shared, host: &HeadlessHost, command: Command) {
    let result = host.reset().and_then(|()| match command.operation {
        Operation::Action(invocation) => runtime.invoke(invocation),
        Operation::Input(events) => runtime.input(events),
        Operation::Resize(size) => runtime.resize(size),
    });
    if let Err(error) = publish(runtime, shared) {
        crate::diagnostics::log_error(&error.to_string());
    }
    if let Err(error) = shared.complete(command.id, result.err().map(|error| error.to_string())) {
        crate::diagnostics::log_error(&error.to_string());
    }
}

fn publish(runtime: &Runtime, shared: &Shared) -> io::Result<()> {
    shared.update(snapshot::capture(runtime))
}

#[cfg(test)]
mod tests;
