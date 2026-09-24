//! Purpose: Tests for the host each platform installs: Linux reads its own
//! `/proc` and signals or renices a process by the identity its table
//! showed; every other platform has none yet and keeps the text report.

use super::system_host;

#[cfg(target_os = "linux")]
mod linux {
    use std::os::unix::process::ExitStatusExt;

    use super::system_host;
    use crate::models::{
        ControlError, Niceness, PriorityChange, ProcessHost, ProcessKey, Sample, Signal, Start,
        StartTime,
    };
    use crate::test_support::sleeper::Sleeper;

    fn sample(host: &dyn ProcessHost) -> Sample {
        let mut reader = host
            .reader()
            .unwrap_or_else(|error| panic!("a reader: {error}"));
        reader
            .sample()
            .unwrap_or_else(|error| panic!("a sample: {error}"))
    }

    /// The key the table shows for `child`, as a dialog would freeze it.
    fn listed(host: &dyn ProcessHost, child: &Sleeper) -> ProcessKey {
        let processes = sample(host).processes;
        let row = processes.iter().find(|row| row.key.pid() == child.pid());
        row.expect("the child is listed").key
    }

    #[test]
    fn linux_has_a_system_host_whose_reader_lists_this_process() {
        let host = system_host().expect("Linux has a process table");
        let own = std::process::id();
        assert!(sample(host.as_ref())
            .processes
            .iter()
            .any(|process| process.key.pid().get() == own));
    }

    /// The host signals by the identity the table showed, and renices only
    /// the process that still has it.
    #[test]
    fn the_system_host_controls_the_process_its_table_listed() {
        let host = system_host().expect("Linux has a process table");
        let mut child = Sleeper::spawn();
        let key = listed(host.as_ref(), &child);
        let newcomer = ProcessKey::new(child.pid(), Start::Known(StartTime::new(u64::MAX)));
        let renice = PriorityChange::Nice(Niceness::MAX);
        assert_eq!(
            host.set_priority(newcomer, renice),
            Err(ControlError::Replaced)
        );
        assert_eq!(host.signal(key, Signal::Terminate), Ok(()));
        assert_eq!(child.wait().signal(), Some(libc::SIGTERM));
    }
}

#[cfg(not(target_os = "linux"))]
#[test]
fn other_platforms_have_no_system_host_yet() {
    assert!(system_host().is_none());
}
