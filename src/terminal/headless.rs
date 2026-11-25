//! Purpose: A private terminal for external programs in an automation session.

use std::{cell::RefCell, io, marker::PhantomData, process::Command, rc::Rc};

mod child;
mod host;
mod keys;
mod master;
mod process;
mod responses;
mod size;
mod state;
mod streams;

pub(crate) use host::HeadlessHost;
pub(crate) use state::{ExternalState, HeadlessSnapshot};

thread_local! {
    static ACTIVE: RefCell<Option<HeadlessHost>> = const { RefCell::new(None) };
}

/// Restores the previous host on the same engine thread when its scope ends.
pub(crate) struct HeadlessScope {
    previous: Option<HeadlessHost>,
    thread: PhantomData<Rc<()>>,
}

impl Drop for HeadlessScope {
    fn drop(&mut self) {
        ACTIVE.with(|active| active.replace(self.previous.take()));
    }
}

/// A command's merged terminal output and portable process exit code.
#[derive(Debug)]
pub(crate) struct ExternalOutcome {
    pub code: u32,
    pub output: Vec<u8>,
}

pub(crate) fn cancelled() -> Option<bool> {
    ACTIVE.with(|active| active.borrow().as_ref().map(HeadlessHost::is_cancelled))
}

pub(crate) fn installed() -> bool {
    ACTIVE.with(|active| active.borrow().is_some())
}

pub(crate) fn viewport() -> Option<ratatui::layout::Size> {
    ACTIVE.with(|active| active.borrow().as_ref().map(HeadlessHost::size))
}

pub(crate) fn resize(width: u16, height: u16) -> io::Result<()> {
    installed_host()?.resize(width, height)
}

pub(crate) fn run(command: &Command) -> io::Result<ExternalOutcome> {
    run_with_timeout(command, None)
}

pub(crate) fn run_with_timeout(
    command: &Command,
    timeout: Option<std::time::Duration>,
) -> io::Result<ExternalOutcome> {
    let host = installed_host()?;
    process::run(&host, command, timeout, child::Descendants::Owned)
}

pub(crate) fn run_detached(
    command: &Command,
    timeout: std::time::Duration,
) -> io::Result<ExternalOutcome> {
    process::run(
        &installed_host()?,
        command,
        Some(timeout),
        child::Descendants::Detached,
    )
}

fn installed_host() -> io::Result<HeadlessHost> {
    ACTIVE
        .with(|active| active.borrow().clone())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotConnected,
                "No private terminal is installed",
            )
        })
}

#[cfg(test)]
mod tests;
