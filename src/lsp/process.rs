//! Purpose: Own a language-server process tree and isolated reader/writer/stderr threads.

use crate::config::ServerSettings;
use serde_json::Value;
use std::{
    io,
    path::Path,
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver, SyncSender},
    thread::{self, JoinHandle},
};

mod group;
mod threads;

pub(super) use group::{configure, exited, terminate};
use threads::{drain_stderr, reader, writer};

pub(super) enum Signal {
    Message(Value),
    Closed,
    Failed,
}

pub(super) struct Process {
    child: Child,
    outgoing: Option<SyncSender<Value>>,
    incoming: Option<Receiver<Signal>>,
    threads: Vec<JoinHandle<()>>,
    #[cfg(windows)]
    job: Option<std::os::windows::io::OwnedHandle>,
}

impl Process {
    pub fn start(settings: &ServerSettings, root: &Path) -> io::Result<Self> {
        let mut command = Command::new(&settings.command);
        command
            .args(&settings.args)
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        configure(&mut command);
        let child = command.spawn()?;
        let mut process = Self {
            child,
            outgoing: None,
            incoming: None,
            threads: Vec::new(),
            #[cfg(windows)]
            job: None,
        };
        #[cfg(windows)]
        {
            process.job = Some(crate::windows::process_tree_job(&process.child)?);
        }
        process.start_threads()?;
        crate::diagnostics::log_action("LSP process started");
        Ok(process)
    }

    fn start_threads(&mut self) -> io::Result<()> {
        let (input, output, stderr) = self.pipes()?;
        self.spawn_threads(input, output, stderr)
    }

    fn pipes(
        &mut self,
    ) -> io::Result<(
        std::process::ChildStdout,
        std::process::ChildStdin,
        std::process::ChildStderr,
    )> {
        // `start` configures all three as piped and takes them once.
        let missing = || io::Error::other("LSP process pipe is not connected");
        let input = self.child.stdout.take().ok_or_else(missing)?;
        let output = self.child.stdin.take().ok_or_else(missing)?;
        let stderr = self.child.stderr.take().ok_or_else(missing)?;
        Ok((input, output, stderr))
    }

    fn spawn_threads(
        &mut self,
        input: std::process::ChildStdout,
        output: std::process::ChildStdin,
        stderr: std::process::ChildStderr,
    ) -> io::Result<()> {
        let (events, incoming) = mpsc::sync_channel(32);
        let (outgoing, messages) = mpsc::sync_channel(8);
        self.incoming = Some(incoming);
        self.outgoing = Some(outgoing);
        let errors = events.clone();
        self.threads.push(
            thread::Builder::new()
                .name("ffm-lsp-read".into())
                .spawn(move || reader(input, &events))?,
        );
        self.threads.push(
            thread::Builder::new()
                .name("ffm-lsp-write".into())
                .spawn(move || writer(output, &messages, &errors))?,
        );
        self.threads.push(
            thread::Builder::new()
                .name("ffm-lsp-stderr".into())
                .spawn(move || drain_stderr(stderr))?,
        );
        Ok(())
    }

    pub fn send(&self, value: Value) -> Result<(), mpsc::TrySendError<Value>> {
        match &self.outgoing {
            Some(sender) => sender.try_send(value),
            None => Err(mpsc::TrySendError::Disconnected(value)),
        }
    }

    pub fn poll(&self) -> Option<Signal> {
        match self.incoming.as_ref()?.try_recv() {
            Ok(event) => Some(event),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Signal::Closed),
        }
    }

    pub fn exited(&mut self) -> bool {
        exited(&mut self.child)
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        self.outgoing.take();
        self.incoming.take(); // Unblock a reader waiting on a full event channel.
        #[cfg(windows)]
        {
            self.job.take();
        }
        terminate(&mut self.child);
        for task in self.threads.drain(..) {
            if task.join().is_err() {
                crate::diagnostics::log_error("LSP I/O task panicked");
            }
        }
        crate::diagnostics::log_action("LSP process and I/O tasks stopped");
    }
}

#[cfg(test)]
mod tests;
