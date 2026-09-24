//! Purpose: Tests for the offered signals: their names, purposes and order.

use super::Signal;

#[cfg(unix)]
#[test]
fn every_posix_signal_is_offered_sigterm_first() {
    let names: Vec<&str> = Signal::OFFERED.iter().map(|signal| signal.name()).collect();
    assert_eq!(
        names,
        [
            "SIGTERM", "SIGKILL", "SIGHUP", "SIGINT", "SIGQUIT", "SIGABRT", "SIGSTOP", "SIGCONT",
            "SIGTSTP", "SIGUSR1", "SIGUSR2", "SIGALRM", "SIGWINCH",
        ]
    );
}

#[cfg(not(unix))]
#[test]
fn no_signal_is_offered_off_unix() {
    assert!(Signal::OFFERED.is_empty());
}

#[test]
fn each_signal_has_its_own_purpose() {
    let every = [
        Signal::Terminate,
        Signal::Kill,
        Signal::Hangup,
        Signal::Interrupt,
        Signal::Quit,
        Signal::Abort,
        Signal::Stop,
        Signal::Continue,
        Signal::TerminalStop,
        Signal::User1,
        Signal::User2,
        Signal::Alarm,
        Signal::WindowChange,
    ];
    let mut purposes: Vec<&str> = every.iter().map(|signal| signal.purpose()).collect();
    purposes.sort_unstable();
    purposes.dedup();
    assert_eq!(purposes.len(), every.len());
    assert_eq!(Signal::Terminate.purpose(), "ask to terminate");
    assert_eq!(Signal::Kill.purpose(), "end at once; cannot be caught");
}
