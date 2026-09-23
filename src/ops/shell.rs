//! Shell command execution.

use std::io::{Read, Write};
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};
use std::{io, path::Path};

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::command::ShellCommand;

#[cfg(windows)]
mod console;
mod error;

pub(crate) use error::ShellError;

/// Start an interactive shell without waiting for it or sharing the TUI's I/O.
pub(super) fn spawn_console(command: &ShellCommand, current_dir: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        console::spawn(command, current_dir).map(drop)
    }
    #[cfg(not(windows))]
    {
        let _ = (command, current_dir);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "separate consoles are only supported on Windows",
        ))
    }
}

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Wall-clock cap on a `:` shell command. The event loop is blocked while the
/// child runs and raw mode has cleared ISIG, so Ctrl+C is just a byte in the
/// input buffer — without a cap, `ping` or `tail -f` freezes FFM for good and
/// the only way out is `kill` from another terminal.
const SHELL_TIMEOUT: Duration = Duration::from_secs(10);

/// How often the wait loop looks at the child and at the keyboard.
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Minimum time the pipe readers get once the shell has exited, so a command
/// that finishes right on the deadline still reports what it printed.
pub(super) const DRAIN_GRACE: Duration = Duration::from_millis(200);

/// Pipe read granularity. Output is forwarded in chunks rather than read to
/// EOF, so an abandoned reader still leaves everything it already sent usable.
const CHUNK_SIZE: usize = 8 * 1024;

#[cfg(any(test, windows))]
pub(crate) const fn windows_shell_args(command: &str) -> [&str; 3] {
    ["/D", "/C", command]
}

pub(crate) fn run_shell_command(command: &str, current_dir: &Path) -> Result<Output, ShellError> {
    run_shell_command_with_timeout(command, current_dir, SHELL_TIMEOUT)
}

/// Run `command` through the platform shell, giving up after `timeout` or as
/// soon as the user presses Esc / Ctrl+C.
pub(crate) fn run_shell_command_with_timeout(
    command: &str,
    current_dir: &Path,
    timeout: Duration,
) -> Result<Output, ShellError> {
    run_until(command, current_dir, timeout, cancel_requested)
}

/// [`run_shell_command_with_timeout`], asking `cancel_requested` rather than
/// the keyboard whether the user wants it stopped.
fn run_until(
    command: &str,
    current_dir: &Path,
    timeout: Duration,
    mut cancel_requested: impl FnMut() -> bool,
) -> Result<Output, ShellError> {
    let mut child = spawn_shell(command, current_dir)
        .map_err(|error| ShellError::launching_in(current_dir, error))?;
    // Drain both pipes on their own threads: a child that fills the pipe buffer
    // (64 KiB on Linux) would block on write and look exactly like a hang.
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());

    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(ShellError::Launch)? {
            break status;
        }
        if let Some(reason) = stop_reason(deadline, timeout, &mut cancel_requested) {
            // Killing the whole group closes the pipe write ends the
            // grandchildren inherited, so the readers below reach EOF and end
            // on their own instead of blocking for the rest of their lifetime.
            kill_shell_tree(&mut child);
            return Err(reason);
        }
        std::thread::sleep(POLL_INTERVAL);
    };

    // The shell exited — but anything it backgrounded (`sleep 30 &`, `nohup
    // server &`) inherited the pipe write end, so the readers cannot see EOF
    // until *that* process dies. Reading to EOF here would hand the event loop
    // over to a daemon's lifetime with the terminal still in raw mode, so the
    // same wall-clock budget bounds the drain: past the deadline we report what
    // arrived and leave the reader to end whenever the writer finally closes.
    // The backgrounded process is deliberately left alone — killing it is
    // exactly what the user asked us not to do by backgrounding it.
    let drain_deadline = deadline.max(Instant::now() + DRAIN_GRACE);
    Ok(Output {
        status,
        stdout: collect_until(&stdout, drain_deadline),
        stderr: collect_until(&stderr, drain_deadline),
    })
}

/// Why a still-running command has to be stopped now, if it has to.
///
/// Two reasons, kept apart: a user who pressed Esc one second in must not be
/// told the command ran out of time.
fn stop_reason(
    deadline: Instant,
    timeout: Duration,
    cancel_requested: &mut impl FnMut() -> bool,
) -> Option<ShellError> {
    if Instant::now() >= deadline {
        return Some(ShellError::TimedOut(timeout));
    }
    cancel_requested().then_some(ShellError::Cancelled)
}

/// Gather forwarded chunks until the reader finishes or `deadline` passes.
pub(super) fn collect_until(chunks: &Receiver<Vec<u8>>, deadline: Instant) -> Vec<u8> {
    let mut collected = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        // `recv_timeout(ZERO)` would keep succeeding against a fast writer, so
        // an expired budget ends the collection outright.
        if remaining.is_zero() {
            return collected;
        }
        // Err is either Disconnected (the reader finished — the common case) or
        // Timeout; both mean there is nothing more to wait for.
        let Ok(chunk) = chunks.recv_timeout(remaining) else {
            return collected;
        };
        collected.extend_from_slice(&chunk);
    }
}

fn spawn_shell(command: &str, current_dir: &Path) -> io::Result<Child> {
    #[cfg(windows)]
    let mut shell = {
        use std::os::windows::process::CommandExt;

        let mut shell = Command::new("cmd");
        shell.creation_flags(CREATE_NO_WINDOW);
        let [disable_autorun, execute, command] = windows_shell_args(command);
        // cmd.exe parses shell syntax, not C-runtime argument escaping.
        // Keep the user's inner quotes intact; cmd removes this outer pair.
        shell
            .args([disable_autorun, execute])
            .raw_arg(format!("\"{command}\""));
        shell
    };

    #[cfg(not(windows))]
    let mut shell = {
        use std::os::unix::process::CommandExt;

        let mut shell = Command::new("sh");
        shell.args(["-c", command]);
        // Own process group (setpgid in the child, before exec) so the timeout
        // can kill `sh -c "sleep …"` *and* its sleep grandchild. Killing only
        // the shell leaves the grandchild alive, holding the pipe write end and
        // reparented to init.
        shell.process_group(0);
        shell
    };

    shell
        .current_dir(current_dir)
        // Piped, not inherited: the child must not draw over the TUI, and we
        // still want its output after a kill.
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}

/// Tear down a timed-out / cancelled shell, including Unix grandchildren.
fn kill_shell_tree(child: &mut Child) {
    // The shell leads its own group (set in `spawn_shell`), and killing a
    // whole group has no std equivalent. It is not reaped yet, so its id
    // cannot name a stranger's group.
    #[cfg(unix)]
    let _ = crate::platform::kill_process_group(child);
    let _ = child.kill();
    let _ = child.wait();
}

/// Read a child pipe on a background thread, forwarding it chunk by chunk.
///
/// Chunks rather than `read_to_end` so [`collect_until`] can stop waiting at any
/// point and still keep what already arrived. A read error ends the stream,
/// exactly like `Command::output` would. The thread also stops as soon as the
/// receiver is gone, so an abandoned reader costs one blocked read, not a
/// permanently spinning thread.
fn drain<R: Read + Send + 'static>(source: Option<R>) -> Receiver<Vec<u8>> {
    drain_echoing(source, io::sink())
}

/// [`drain`], also copying each chunk to `echo` as it arrives, so a program
/// that owns the terminal still shows its messages live while the caller gets
/// to keep them.
pub(super) fn drain_echoing<R, W>(source: Option<R>, mut echo: W) -> Receiver<Vec<u8>>
where
    R: Read + Send + 'static,
    W: Write + Send + 'static,
{
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let Some(mut source) = source else { return };
        let mut chunk = vec![0_u8; CHUNK_SIZE];
        while let Ok(read) = source.read(&mut chunk) {
            if read == 0 {
                return;
            }
            // Best effort: a lost echo leaves the kept copy intact.
            let _ = echo.write_all(&chunk[..read]).and_then(|()| echo.flush());
            if sender.send(chunk[..read].to_vec()).is_err() {
                return;
            }
        }
    });
    receiver
}

mod tty;
use tty::cancel_requested;

/// Whether `event` is the cancel gesture: Esc or Ctrl+C, on key press only.
///
/// Other keys are dropped on purpose, so keystrokes typed at a running command
/// cannot replay against the panels once it finishes. The decision is pure so
/// it can be pinned by unit tests; the terminal polling around it lives in
/// [`tty`], which is nothing but glue.
pub(super) fn is_cancel_event(event: &Event) -> bool {
    let Event::Key(key) = event else {
        return false;
    };
    if key.kind != KeyEventKind::Press {
        return false;
    }
    let ctrl_c = key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL);
    ctrl_c || key.code == KeyCode::Esc
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_outcomes;
