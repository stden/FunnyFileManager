//! Purpose: Opening the task list: the live viewer where a process host is
//! installed, and the `ps` or `tasklist` report where none is.
//!
//! Every entry point opens it here: `Ctrl+W`, F9 > Commands > Task list, and
//! the Change drive dialog's Process list row.
//!
//! What the list's keys do to processes — the confirmations, the signals and
//! the renices — is [`control`].

use std::io;

use crate::models::{App, Overlay, Pid, TaskList};
use crate::prompt::Privilege;

pub mod control;
mod report;

/// Opens the task list: a new session of the live viewer when a process host
/// is installed, otherwise the text report, whose failure is the caller's to
/// show.
pub fn open(app: &mut App) -> io::Result<()> {
    let own = Pid::new(std::process::id());
    let (Some(_), Some(own)) = (app.task_host.host(), own) else {
        return report::open(app);
    };
    let session = app.task_host.next_session();
    let list = TaskList::new(session, own, Privilege::current());
    app.set_overlay(Overlay::Tasks(Box::new(list)));
    Ok(())
}

#[cfg(test)]
mod tests;
