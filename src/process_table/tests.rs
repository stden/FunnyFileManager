//! Purpose: Tests for the host each platform installs: Linux and Windows read
//! live system processes and network counters, verify identity before priority
//! or terminate calls, and expose a `NetProber`.

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

    fn listed(host: &dyn ProcessHost, child: &Sleeper) -> ProcessKey {
        let processes = sample(host).processes;
        let row = processes.iter().find(|row| row.key.pid() == child.pid());
        row.expect("the child is listed").key
    }

    #[test]
    fn linux_has_a_system_host_whose_reader_lists_this_process() {
        let host = system_host().expect("Linux has a process table");
        assert!(host.net_prober().is_some());
        let own = std::process::id();
        assert!(sample(host.as_ref())
            .processes
            .iter()
            .any(|process| process.key.pid().get() == own));
    }

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

#[cfg(windows)]
mod windows {
    use super::system_host;
    use crate::models::{
        ControlError, PriorityChange, ProcessHost, ProcessKey, Sample, SettableClass, Signal,
        Start, StartTime,
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

    #[test]
    fn windows_has_a_system_host_with_live_processes_network_and_prober() {
        let host = system_host().expect("Windows has a process table");
        assert!(host.net_prober().is_some());
        let snap = sample(host.as_ref());
        let own = std::process::id();
        assert!(snap.processes.iter().any(|p| p.key.pid().get() == own));
        assert_ne!(snap.network.adapters.len(), 0);
    }

    #[test]
    fn windows_system_host_verifies_identity_renices_and_terminates_child() {
        let host = system_host().expect("Windows has a process table");
        let mut child = Sleeper::spawn();
        let key = sample(host.as_ref())
            .processes
            .iter()
            .find(|row| row.key.pid() == child.pid())
            .expect("the child is listed")
            .key;
        let newcomer = ProcessKey::new(child.pid(), Start::Known(StartTime::new(u64::MAX)));
        assert_eq!(
            host.set_priority(newcomer, PriorityChange::Class(SettableClass::BelowNormal)),
            Err(ControlError::Replaced)
        );
        assert_eq!(
            host.set_priority(key, PriorityChange::Class(SettableClass::BelowNormal)),
            Ok(())
        );
        assert_eq!(host.signal(key, Signal::Terminate), Ok(()));
        assert!(!child.wait().success());
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
#[test]
fn other_platforms_have_no_system_host_yet() {
    assert!(system_host().is_none());
}
