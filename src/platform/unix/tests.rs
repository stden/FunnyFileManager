//! Purpose: Tests for the one place a model signal becomes a system signal
//! number, checked against the C library's own constants.

use super::rustix_signal;
use crate::models::Signal;

#[test]
fn each_signal_maps_to_the_c_library_number() {
    let cases = [
        (Signal::Terminate, libc::SIGTERM),
        (Signal::Kill, libc::SIGKILL),
        (Signal::Hangup, libc::SIGHUP),
        (Signal::Interrupt, libc::SIGINT),
        (Signal::Quit, libc::SIGQUIT),
        (Signal::Abort, libc::SIGABRT),
        (Signal::Stop, libc::SIGSTOP),
        (Signal::Continue, libc::SIGCONT),
        (Signal::TerminalStop, libc::SIGTSTP),
        (Signal::User1, libc::SIGUSR1),
        (Signal::User2, libc::SIGUSR2),
        (Signal::Alarm, libc::SIGALRM),
        (Signal::WindowChange, libc::SIGWINCH),
    ];
    for (signal, number) in cases {
        assert_eq!(rustix_signal(signal).as_raw(), number, "{}", signal.name());
    }
}

#[test]
fn every_offered_signal_has_a_number() {
    assert!(Signal::OFFERED
        .iter()
        .all(|signal| rustix_signal(*signal).as_raw() > 0));
}
