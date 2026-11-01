//! Live Windows smoke tests for process, system, and network platform wrappers.

use std::time::Duration;

use crate::models::tasks::{Ipv4, ProbeOutcome};
use crate::models::{
    ControlError, ControlRefusal, Pid, PriorityChange, ProcessKey, SettableClass, Signal, Start,
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
