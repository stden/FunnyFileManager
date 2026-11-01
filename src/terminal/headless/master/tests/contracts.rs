//! Purpose: Direct contracts for private Unix terminal control and descriptor flags.

use super::super::*;
use nix::fcntl::{fcntl, FcntlArg, OFlag};

#[test]
fn an_empty_private_terminal_has_no_foreground_process_to_signal() {
    let pair = portable_pty::native_pty_system()
        .openpty(PtySize::default())
        .unwrap();
    // Never signal a group unless this private fixture has none to begin with.
    assert_eq!(pair.master.process_group_leader(), None);
    crate::platform::kill_terminal_foreground(&*pair.master).unwrap();
    let master = Master::new(pair.master);
    master.foreground_stop().unwrap();
    assert_eq!(master.size().unwrap(), PtySize::default());
}

#[test]
fn platform_nonblocking_configuration_preserves_other_descriptor_flags() {
    let pair = portable_pty::native_pty_system()
        .openpty(PtySize::default())
        .unwrap();
    let original = pair.master.as_raw_fd().unwrap();
    let before = OFlag::from_bits_retain(fcntl(original, FcntlArg::F_GETFL).unwrap());
    assert!(!before.contains(OFlag::O_NONBLOCK));
    let configured = crate::platform::nonblocking_terminal_fd(&*pair.master).unwrap();
    let after = OFlag::from_bits_retain(fcntl(configured, FcntlArg::F_GETFL).unwrap());
    assert_eq!(configured, original);
    assert_eq!(after, before | OFlag::O_NONBLOCK);
}

#[test]
fn master_reader_descriptor_is_nonblocking_and_stays_owned() {
    let pair = portable_pty::native_pty_system()
        .openpty(PtySize::default())
        .unwrap();
    let master = Master::new(pair.master);
    let reader = master.nonblocking_fd().unwrap();
    let flags = OFlag::from_bits_retain(fcntl(reader, FcntlArg::F_GETFL).unwrap());
    assert!(flags.contains(OFlag::O_NONBLOCK));
    assert_eq!(master.size().unwrap(), PtySize::default());
    // Both master and slave remain owned until this test's scope ends.
}
