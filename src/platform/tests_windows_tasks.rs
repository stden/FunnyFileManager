//! Live Windows smoke tests for process, system, and network platform wrappers.

use std::os::windows::process::CommandExt;
use std::time::Duration;

use crate::models::tasks::{Ipv4, ProbeOutcome};
use crate::models::{
    ControlError, ControlRefusal, Pid, PriorityChange, ProcessKey, SettableClass, Signal, Start,
    StartTime,
};

#[test]
fn windows_system_and_process_queries_find_the_current_process() {
    assert!(super::now_filetime_ticks() > 0);
    let sys = super::read_system_sample().expect("system sample");
    assert_ne!(sys.cores.len(), 0);
    assert!(sys.memory.is_some());
    assert!(sys.uptime.is_some());

    let mut owners = super::WinOwnerCache::default();
    let (procs, _) = super::list_processes(&mut owners).expect("process list");
    let own = std::process::id();
    let me = procs
        .iter()
        .find(|p| p.key.pid().get() == own)
        .expect("current process listed");
    assert!(matches!(me.key.start(), Start::Known(_)));
    assert!(me.memory.is_some());
}

#[test]
fn windows_network_queries_enumerate_interfaces_and_loopback_icmp() {
    let endpoints = super::read_net_endpoints();
    let adapters = super::read_network_adapters(&endpoints.ipv4_by_index);
    assert_ne!(adapters.len(), 0);

    let ping = super::icmp_ping(Ipv4::new(127, 0, 0, 1), Duration::from_millis(500));
    assert!(
        matches!(ping, ProbeOutcome::Ok(_)),
        "loopback ping: {ping:?}"
    );
}

#[test]
fn windows_control_wrappers_refuse_own_and_unverifiable_targets() {
    let own = Pid::new(std::process::id()).expect("valid own pid");
    let unknown_key = ProcessKey::new(own, Start::Unknown);
    assert_eq!(
        super::terminate_process(unknown_key, Signal::Terminate, own),
        Err(ControlError::Refused(ControlRefusal::OwnProcess))
    );
    let other_pid = Pid::new(own.get().saturating_add(1)).expect("valid pid");
    let unverifiable = ProcessKey::new(other_pid, Start::Unknown);
    assert_eq!(
        super::set_process_priority(unverifiable, PriorityChange::Class(SettableClass::Normal)),
        Err(ControlError::Unverifiable)
    );
}

struct ChildGuard(std::process::Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        drop(self.0.kill());
        drop(self.0.wait());
    }
}

fn spawn_listed_child() -> (ChildGuard, ProcessKey, StartTime) {
    let child = ChildGuard(
        std::process::Command::new("ping.exe")
            .args(["-n", "30", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW)
            .spawn()
            .expect("spawn child"),
    );
    let child_pid = child.0.id();
    let mut owners = super::WinOwnerCache::default();
    let (procs, _) = super::list_processes(&mut owners).expect("process list");
    let entry = procs
        .iter()
        .find(|p| p.key.pid().get() == child_pid)
        .expect("child process listed");
    let Start::Known(actual) = entry.key.start() else {
        panic!("child must have known StartTime");
    };
    (child, entry.key, actual)
}

#[test]
fn windows_control_pins_child_identity_updates_priority_and_terminates_cleanly() {
    let (mut child, key, actual) = spawn_listed_child();
    let forged_stamp = if actual == StartTime::new(1) {
        StartTime::new(2)
    } else {
        StartTime::new(1)
    };
    let forged = ProcessKey::new(key.pid(), Start::Known(forged_stamp));
    let own = Pid::new(std::process::id()).expect("own pid");

    assert_eq!(
        super::set_process_priority(forged, PriorityChange::Class(SettableClass::BelowNormal)),
        Err(ControlError::Replaced)
    );
    assert_eq!(
        super::terminate_process(forged, Signal::Terminate, own),
        Err(ControlError::Replaced)
    );
    assert_eq!(
        super::set_process_priority(key, PriorityChange::Class(SettableClass::BelowNormal)),
        Ok(())
    );
    assert_eq!(
        super::terminate_process(key, Signal::Terminate, own),
        Ok(())
    );
    drop(child.0.wait());
    assert!(matches!(
        super::set_process_priority(key, PriorityChange::Class(SettableClass::Normal)),
        Err(ControlError::Gone | ControlError::PermissionDenied)
    ));
}

#[test]
fn process_table_system_host_samples_probes_and_validates_control() {
    let host = crate::process_table::system_host().expect("windows host");
    let mut reader = host.reader().expect("reader");
    let sample = reader.sample().expect("sample");
    assert!(!sample.processes.is_empty());
    assert!(host.net_prober().is_some());
    let own = Pid::new(std::process::id()).expect("own pid");
    let key = ProcessKey::new(own, Start::Unknown);
    assert!(host.signal(key, Signal::Terminate).is_err());
    assert!(host
        .set_priority(key, PriorityChange::Class(SettableClass::Normal))
        .is_err());
}
