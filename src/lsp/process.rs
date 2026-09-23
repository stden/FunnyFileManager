//! Purpose: Own a language-server process tree and isolated reader/writer/stderr threads.

use super::transport;
use crate::config::ServerSettings;
use serde_json::Value;
use std::{
    io::{self, BufReader, Read},
    path::Path,
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver, SyncSender},
    thread::{self, JoinHandle},
};

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
            process.job = Some(crate::windows::lsp_job(&process.child)?);
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

fn reader(input: impl Read, events: &SyncSender<Signal>) {
    let mut input = BufReader::new(input);
    loop {
        let event = match transport::read_frame(&mut input) {
            Ok(Some(message)) => Signal::Message(message),
            Ok(None) => Signal::Closed,
            Err(_) => Signal::Failed,
        };
        let finished = !matches!(event, Signal::Message(_));
        if events.send(event).is_err() || finished {
            return;
        }
    }
}

fn writer(mut output: impl io::Write, messages: &Receiver<Value>, events: &SyncSender<Signal>) {
    for message in messages {
        if transport::write_frame(&mut output, &message).is_err() {
            // The receiver may already be gone during normal shutdown.
            drop(events.send(Signal::Failed));
            return;
        }
    }
}

fn drain_stderr(mut stderr: impl Read) {
    let mut buffer = [0; 1024];
    loop {
        match stderr.read(&mut buffer) {
            Ok(0) => return,
            Ok(bytes) => crate::diagnostics::log_error(&format!(
                "LSP stderr received ({bytes} bytes; content redacted)"
            )),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => {
                crate::diagnostics::log_error("LSP stderr read failed");
                return;
            }
        }
    }
}

#[cfg(windows)]
pub(super) fn configure(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: only the editor owns a terminal.
}

#[cfg(unix)]
pub(super) fn configure(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(unix)]
pub(super) fn exited(child: &mut Child) -> bool {
    use rustix::process::{waitid, Pid, WaitId, WaitIdOptions};
    // NOWAIT retains the leader's PID until terminate has killed its entire group.
    waitid(
        WaitId::Pid(Pid::from_child(child)),
        WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
    )
    .is_ok_and(|status| status.is_some())
}

#[cfg(windows)]
pub(super) fn exited(child: &mut Child) -> bool {
    child.try_wait().is_ok_and(|status| status.is_some())
}

#[cfg(unix)]
pub(super) fn terminate(child: &mut Child) {
    // The leader has not been reaped: its PID cannot belong to an unrelated group.
    // A group that is already gone needs no signal, and `reap` kills the
    // leader either way.
    drop(crate::platform::kill_process_group(child));
    reap(child);
}

#[cfg(windows)]
pub(super) fn terminate(child: &mut Child) {
    reap(child);
}

fn reap(child: &mut Child) {
    if !child.try_wait().is_ok_and(|status| status.is_some()) {
        // The job/group may have terminated the process between try_wait and kill.
        drop(child.kill());
    }
    if child.wait().is_err() {
        crate::diagnostics::log_error("LSP process reap failed");
    }
}

#[cfg(test)]
mod tests;
