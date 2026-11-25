//! The terminal wait: what counts as a hang-up, and how fast it is noticed.
//!
//! A socket pair stands in for the terminal. It reports the same `poll(2)`
//! status bits a closed pty does, and unlike a pty it needs no controlling
//! terminal, so these run anywhere the suite does.

use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use signal_hook::consts::SIGWINCH;

use super::{lost, wait_for_fd, TerminalLost};

/// Long enough that a wait which really blocks would be obvious, short enough
/// that the suite never notices.
const BRIEF: Duration = Duration::from_millis(20);
const SECOND: Duration = Duration::from_secs(1);
/// The longest a resize that was already pending may take to end its wait. The
/// wait returns at once; this bound only leaves room for a loaded machine, and
/// it is well inside [`SECOND`], the budget the wait was given.
const AT_ONCE: Duration = Duration::from_millis(250);

/// Waits take turns. Each one drains the process's single SIGWINCH pipe, so a
/// wait running beside the resize test could swallow the resize it raised. A
/// waiting test takes its turn before it starts its clock.
fn turn() -> MutexGuard<'static, ()> {
    static TURNS: Mutex<()> = Mutex::new(());
    TURNS.lock().unwrap_or_else(PoisonError::into_inner)
}

#[test]
fn only_the_hang_up_bits_end_the_session() {
    assert!(!lost(0), "an idle terminal is not a lost one");
    assert!(!lost(libc::POLLIN), "a key waiting is not a lost terminal");
    assert!(lost(libc::POLLHUP));
    assert!(lost(libc::POLLERR));
    assert!(lost(libc::POLLNVAL));
    assert!(
        lost(libc::POLLIN | libc::POLLHUP),
        "a hang-up counts even with unread bytes in front of it"
    );
}

/// An idle terminal spends the whole budget here, so nothing is owed: the
/// caller may ask again at once and the loop still runs at one poll per
/// period.
#[test]
fn a_live_peer_spends_the_whole_budget() {
    let _turn = turn();
    let (ours, _theirs) = UnixStream::pair().expect("a socket pair");

    let started = Instant::now();
    let unspent = wait_for_fd(ours.as_raw_fd(), BRIEF).expect("a live peer is not a hang-up");

    assert!(
        started.elapsed() >= BRIEF / 2,
        "it returned without waiting"
    );
    assert!(
        unspent < BRIEF / 2,
        "an idle wait has nothing left over: {unspent:?}"
    );
}

/// Input returns at once — and the budget it did not spend comes back with
/// it. crossterm's reader is edge-triggered, so bytes it left queued fire
/// this probe without producing an event; the caller sleeps the remainder
/// rather than asking again immediately and pinning a core.
#[test]
fn bytes_waiting_return_the_budget_they_did_not_spend() {
    let _turn = turn();
    let (ours, mut theirs) = UnixStream::pair().expect("a socket pair");
    io::Write::write_all(&mut theirs, b"q").expect("send a keystroke");

    let started = Instant::now();
    let unspent = wait_for_fd(ours.as_raw_fd(), BRIEF).expect("input is not a hang-up");

    assert!(
        started.elapsed() < BRIEF / 2,
        "input must not be waited out"
    );
    assert!(
        unspent > BRIEF / 2,
        "the caller was left nothing to sleep, so it would spin: {unspent:?}"
    );
}

#[test]
fn a_closed_peer_is_reported_at_once_instead_of_spinning() {
    let _turn = turn();
    let (ours, theirs) = UnixStream::pair().expect("a socket pair");
    drop(theirs);

    let started = Instant::now();
    let error = wait_for_fd(ours.as_raw_fd(), Duration::from_secs(5))
        .expect_err("a closed peer is a lost terminal");

    assert!(TerminalLost::caused(&error), "wrong error: {error}");
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "the hang-up must be reported without waiting the timeout out"
    );
}

/// A resize that landed while the loop was busy is already pending when the
/// next wait starts. The wait used to sleep its whole budget first, because
/// only the terminal was watched; now the pending resize ends it at once, and
/// only once: the next wait spends its budget again.
#[test]
fn a_resize_pending_before_the_wait_ends_it_at_once() {
    let _turn = turn();
    let (ours, _theirs) = UnixStream::pair().expect("a socket pair");
    // The first wait installs the SIGWINCH pipe, and a signal raised before
    // that is not caught; this wait only sets the pipe up.
    wait_for_fd(ours.as_raw_fd(), Duration::ZERO).expect("an idle terminal is not a hang-up");
    signal_hook::low_level::raise(SIGWINCH).expect("raise SIGWINCH");

    let started = Instant::now();
    wait_for_fd(ours.as_raw_fd(), SECOND).expect("a resize is not a hang-up");
    assert!(
        started.elapsed() < AT_ONCE,
        "a resize that was already pending waited out the budget: {:?}",
        started.elapsed()
    );

    let started = Instant::now();
    wait_for_fd(ours.as_raw_fd(), BRIEF).expect("an idle terminal is not a hang-up");
    assert!(
        started.elapsed() >= BRIEF / 2,
        "the same resize woke the next wait too"
    );
}

#[test]
fn the_marker_is_matched_by_type_not_by_kind_or_message() {
    assert!(TerminalLost::caused(&TerminalLost::error()));
    assert!(
        !TerminalLost::caused(&io::Error::new(
            io::ErrorKind::BrokenPipe,
            "the controlling terminal disconnected"
        )),
        "an unrelated error that merely reads the same must not end the session"
    );
    assert!(
        !TerminalLost::caused(&io::Error::new(io::ErrorKind::UnexpectedEof, "done")),
        "the recording-exhausted sentinel is not a lost terminal"
    );
}
