//! Purpose: Background FFM sessions expose the real application without acquiring a terminal.
#![allow(dead_code, reason = "test")]
use std::{ffi::OsString, io};

pub mod catalog;
mod dispatch;
pub mod options;
mod protocol;
mod server;
mod session;
mod snapshot;
mod transport;

pub fn arguments(args: &[OsString]) -> Option<&[OsString]> {
    let (first, remaining) = args.split_first()?;
    (first == "--automation").then_some(remaining)
}

pub fn run(args: &[OsString]) -> io::Result<()> {
    server::run(options::Options::parse(args)?)
}

#[cfg(test)]
mod tests;
