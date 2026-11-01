//! Waiting for the controlling terminal, and noticing when it is gone.
//!
//! crossterm normally owns the wait inside `event::poll`, but its Unix source
//! reads the descriptor in a loop whose only non-event exit is `WouldBlock`
//! (crossterm 0.29, `src/event/source/unix/mio.rs`). A hung-up terminal — the
//! window closed, SSH dropped, `tmux kill-session` — is reported readable and
//! reads zero bytes forever, so that loop never returns and the process spins
//! on a core until something kills it. Doing the wait here lets the hang-up be
//! seen as a `poll(2)` status bit, before crossterm ever touches the
//! descriptor.

use std::io;
use std::time::Duration;

#[cfg(unix)]
mod resize;

/// The controlling terminal disappeared.
///
/// Carried inside an `io::Error` so the event loop can recognise it by type:
/// the loop's own "recording exhausted" sentinel is an `io::Error` too, and a
/// comparison on kind or message would confuse the two.
#[derive(Debug)]
pub struct TerminalLost;

impl std::fmt::Display for TerminalLost {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("the controlling terminal disconnected")
    }
}

impl std::error::Error for TerminalLost {}

impl TerminalLost {
    /// The error the wait returns once the terminal is gone.
    #[cfg(any(unix, test))]
    pub fn error() -> io::Error {
        io::Error::new(io::ErrorKind::BrokenPipe, Self)
    }

    /// Whether `error` carries this marker.
    pub fn caused(error: &io::Error) -> bool {
        error
            .get_ref()
            .is_some_and(<dyn std::error::Error + Send + Sync>::is::<Self>)
    }
}

/// Wait up to `timeout` for the terminal, and return how much of that budget
/// was left unspent. A resize ends the wait early too, whenever it arrived;
/// see `resize`.
///
/// crossterm is handed no time at all afterwards — it only has to look, and it
/// must never be left blocking on a descriptor that may hang up under it. The
/// unspent remainder is what the caller owes the loop instead: crossterm's
/// reader is edge-triggered, so bytes already queued when it last returned
/// make this probe fire at once while crossterm reports nothing, and a caller
/// that asked again immediately would spin on a core. Sleeping the remainder
/// keeps that case at the one poll per `timeout` it has always been.
///
/// A caught SIGHUP or SIGTERM reads as a lost terminal too: it is the same
/// news, and it may come before `poll` can see the hang-up, or without one.
#[cfg(unix)]
pub fn wait(timeout: Duration) -> io::Result<Duration> {
    if crate::terminal::hung_up() {
        return Err(TerminalLost::error());
    }
    if crossterm_reads_stdin() {
        wait_for_fd(libc::STDIN_FILENO, timeout)
    } else {
        Ok(Duration::ZERO)
    }
}

/// Whether crossterm reads stdin, as it was when the first wait asked.
///
/// crossterm reads stdin only when stdin *is* the terminal; otherwise it opens
/// /dev/tty, which this probe would not be watching, so that case is left to
/// crossterm unchanged. crossterm decides once, when its reader starts, and
/// never looks again — so neither may this. A hung-up terminal fails `isatty`
/// (Linux answers `EIO`), and asking afresh would take the dead terminal for
/// "not a terminal" and hand it straight to the read that never returns.
#[cfg(unix)]
fn crossterm_reads_stdin() -> bool {
    use std::io::IsTerminal;

    static READS_STDIN: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *READS_STDIN.get_or_init(|| io::stdin().is_terminal())
}

/// Windows tears its console's processes down with it and the console API has
/// no equivalent spin, so nothing is owed: crossterm keeps the whole wait.
#[cfg(not(unix))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "shares the Unix signature, where the operation can fail"
)]
pub fn wait(_timeout: Duration) -> io::Result<Duration> {
    Ok(Duration::ZERO)
}

#[cfg(unix)]
pub(super) const fn crossterm_wait(_timeout: Duration) -> Duration {
    Duration::ZERO
}

#[cfg(not(unix))]
pub(super) const fn crossterm_wait(timeout: Duration) -> Duration {
    timeout
}

/// The wait itself, on an explicit descriptor so a test can hand it a socket
/// pair instead of a terminal.
///
/// A resize ends the wait early, whether it comes during the poll (which it
/// interrupts) or before it (the pipe in `resize` is already readable). The
/// zero-timeout `event::poll` that follows lets crossterm report the resize.
#[cfg(unix)]
#[allow(
    unsafe_code,
    reason = "poll(2) on two descriptors has no safe wrapper in std"
)]
fn wait_for_fd(fd: std::os::fd::RawFd, timeout: Duration) -> io::Result<Duration> {
    let mut watched = [watch_input(fd), watch_input(resize::descriptor()?)];
    let millis = i32::try_from(timeout.as_millis()).unwrap_or(i32::MAX);
    let started = std::time::Instant::now();
    // SAFETY: `watched` holds two initialized `pollfd`s and the count passed
    // is two; poll writes only their `revents` fields and reads no other user
    // memory.
    let ready = unsafe { libc::poll(watched.as_mut_ptr(), 2, millis) };
    let unspent = timeout.saturating_sub(started.elapsed());
    // Emptied even when the signal itself interrupted the poll, or the next
    // wait would wake for the same resize again.
    resize::drain()?;
    let [terminal, _resize] = watched;
    if ready < 0 {
        // Interrupted, or the kernel is out of resources. Nothing is known
        // about the terminal, so nothing is owed either.
        return Ok(Duration::ZERO);
    }
    if lost(terminal.revents) {
        return Err(TerminalLost::error());
    }
    Ok(unspent)
}

/// A `poll(2)` request for input on `fd`.
#[cfg(unix)]
fn watch_input(fd: std::os::fd::RawFd) -> libc::pollfd {
    libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    }
}

/// The status bits that mean the far end of the terminal is gone.
///
/// `poll` reports them whether or not they were asked for, and never reports
/// them for a live terminal — not even one with bytes already waiting.
#[cfg(unix)]
const fn lost(revents: libc::c_short) -> bool {
    revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0
}

#[cfg(all(test, unix))]
mod tests;
