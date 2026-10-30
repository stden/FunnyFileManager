//! Purpose: The command lines the reader keeps between samples, so each
//! process's `cmdline` is read once per identity and name rather than on
//! every refresh — htop's default, "Update process names on every refresh"
//! off.
//!
//! Reading `cmdline` takes the target's mmap lock, which a process stuck in
//! the kernel can hold for good; the fewer reads, the fewer chances to hang
//! behind one. `execve` changes the short name, so a process that runs a new
//! program is read again.

use std::collections::HashMap;
use std::mem;

use crate::models::ProcessKey;

/// Command lines by process identity: those the last sample kept, and those
/// this one has kept so far.
///
/// A process the last sample saw but this one does not is dropped when the
/// next sample starts, so the cache never outgrows the process table.
#[derive(Default)]
pub(super) struct CommandCache {
    last: HashMap<ProcessKey, Command>,
    this: HashMap<ProcessKey, Command>,
}

/// A command line, and the short name the process had when it was read.
struct Command {
    name: String,
    line: String,
}

impl CommandCache {
    /// Starts a sample: only what the last one kept can be reused.
    pub(super) fn start_sample(&mut self) {
        self.last = mem::take(&mut self.this);
    }

    /// The command line kept for `key`, if it was read while the process was
    /// still called `name`; kept for the next sample too.
    pub(super) fn reuse(&mut self, key: ProcessKey, name: &str) -> Option<String> {
        let kept = self.last.remove(&key).filter(|kept| kept.name == name)?;
        let line = kept.line.clone();
        self.this.insert(key, kept);
        Some(line)
    }

    /// Keeps `line`, just read for `key` while it was called `name`.
    pub(super) fn keep(&mut self, key: ProcessKey, name: &str, line: &str) {
        let command = Command {
            name: name.to_owned(),
            line: line.to_owned(),
        };
        self.this.insert(key, command);
    }
}
