//! Purpose: The per-process side of the `/proc` reader: every process the
//! table lists, each read from its `stat`, `statm`, `status` and `cmdline`
//! into one row, and why a process is left out of a sample.

use std::fs;
use std::path::Path;
use std::time::Duration;

use crate::directory::owner::Owners;
use crate::models::{Owner, Pid, ProcessKey, ProcessSample, ProcessState, SampleError, Start};

use super::files::{vanished, ProcessFile};
use super::parse::{self, Stat};
use super::{unlisted, ProcFs};

/// Why a process is left out of a sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Skip {
    /// It exited during the scan: left out without a word.
    Vanished,
    /// Its `stat` could not be read or parsed: counted as unreadable.
    Unreadable,
}

impl ProcFs {
    /// Every process `entries` lists, each once, and how many could not be
    /// read. A listing that fails part way fails the sample: a table missing
    /// the rest of it would look complete.
    pub(super) fn processes(
        &mut self,
        entries: fs::ReadDir,
        uptime: Option<Duration>,
    ) -> Result<(Vec<ProcessSample>, u32), SampleError> {
        self.users.forget_failed_lookups();
        self.commands.start_sample();
        let (mut processes, mut unreadable) = (Vec::new(), 0_u32);
        for entry in entries {
            let entry = entry.map_err(|error| unlisted(&error))?;
            let Some(pid) = entry.file_name().to_str().and_then(parse::directory_pid) else {
                continue;
            };
            match self.process(&entry.path(), pid, uptime) {
                Ok(process) => processes.push(process),
                Err(Skip::Unreadable) => unreadable = unreadable.saturating_add(1),
                Err(Skip::Vanished) => {}
            }
        }
        processes.sort_by_key(|process| process.key);
        processes.dedup_by_key(|process| process.key);
        Ok((processes, unreadable))
    }

    /// The process `pid`, whose files are in `dir`.
    fn process(
        &mut self,
        dir: &Path,
        pid: Pid,
        uptime: Option<Duration>,
    ) -> Result<ProcessSample, Skip> {
        let (ticks, page) = (self.ticks, self.page);
        let stat = self
            .read(dir, ProcessFile::Stat, |text| parse::stat(text, pid, ticks))?
            .flatten()
            .ok_or(Skip::Unreadable)?;
        let memory = self
            .read(dir, ProcessFile::Statm, |text| parse::statm(text, page))?
            .flatten();
        let uid = self
            .read(dir, ProcessFile::Status, parse::effective_uid)?
            .flatten();
        let key = ProcessKey::new(pid, Start::Known(stat.start));
        let command = self.command(dir, key, &stat)?;
        Ok(ProcessSample {
            key,
            parent: stat.parent,
            name: stat.name,
            command,
            owner: uid.map_or(Owner::Unknown, |uid| owner(&mut self.users, uid)),
            state: stat.state,
            priority: stat.priority,
            threads: stat.threads,
            cpu_time: stat.cpu_time,
            memory,
            running_for: uptime.map(|uptime| uptime.saturating_sub(stat.started)),
            kind: stat.kind,
        })
    }

    /// The command line of the process `key`: kept from an earlier sample
    /// while its name is unchanged, never read while it sleeps in the kernel
    /// uncached (the read would wait on its mmap lock), and `[name]` when it
    /// has none or it cannot be read.
    fn command(&mut self, dir: &Path, key: ProcessKey, stat: &Stat) -> Result<String, Skip> {
        if let Some(line) = self.commands.reuse(key, &stat.name) {
            return Ok(line);
        }
        let unread = || parse::command_line("", &stat.name);
        if stat.state == ProcessState::DiskSleep {
            return Ok(unread());
        }
        let read = self.read(dir, ProcessFile::Cmdline, |text| {
            parse::command_line(text, &stat.name)
        })?;
        let Some(line) = read else {
            return Ok(unread());
        };
        self.commands.keep(key, &stat.name, &line);
        Ok(line)
    }

    /// `file` of the process whose files are in `dir`, parsed: `Ok(None)`
    /// when it cannot be read, and [`Skip::Vanished`] when that is because
    /// the process has exited.
    fn read<T>(
        &mut self,
        dir: &Path,
        file: ProcessFile,
        parse: impl FnOnce(&str) -> T,
    ) -> Result<Option<T>, Skip> {
        match self.buffer.read(&dir.join(file.name()), file.cap()) {
            Ok(text) => Ok(Some(parse(&text))),
            Err(error) if vanished(&error) => Err(Skip::Vanished),
            Err(_) => Ok(None),
        }
    }
}

/// The user `uid` names: by name, or by number when it has no passwd entry
/// or its lookup failed (asked again next sample).
fn owner(users: &mut Owners, uid: u32) -> Owner {
    users.name_of_uid(uid).map_or(Owner::Uid(uid), Owner::Named)
}
