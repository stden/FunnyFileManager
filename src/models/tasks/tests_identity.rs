//! Purpose: Tests for process identity: the PID range and the key that tells a
//! reused PID apart.

use super::identity::TaskSession;
use super::{Pid, ProcessKey, Start, StartTime};

const LARGEST_PID: u32 = i32::MAX.unsigned_abs();

#[test]
fn a_pid_is_one_through_i32_max() {
    let cases = [
        (0, None),
        (1, Some(1)),
        (4242, Some(4242)),
        (LARGEST_PID, Some(LARGEST_PID)),
        (LARGEST_PID + 1, None),
        (u32::MAX, None),
    ];
    for (raw, expected) in cases {
        assert_eq!(Pid::new(raw).map(Pid::get), expected, "Pid::new({raw})");
    }
}

#[test]
fn a_pid_is_the_same_number_signed_and_unsigned() {
    let largest = Pid::new(LARGEST_PID).expect("in range");
    assert_eq!(largest.raw(), i32::MAX);
    let pid = Pid::new(4242).expect("in range");
    assert_eq!(pid.raw(), 4242);
    assert_eq!(pid.to_string(), "4242");
}

#[test]
fn pids_order_numerically() {
    let small = Pid::new(2).expect("in range");
    let large = Pid::new(LARGEST_PID).expect("in range");
    assert!(small < large);
}

#[test]
fn a_reused_pid_with_a_new_start_is_a_different_key() {
    let pid = Pid::new(4242).expect("in range");
    let first = ProcessKey::new(pid, Start::Known(StartTime::new(100)));
    let newcomer = ProcessKey::new(pid, Start::Known(StartTime::new(900)));
    assert_ne!(first, newcomer);
    assert_eq!(first.pid(), newcomer.pid());
    assert_eq!(first.start(), Start::Known(StartTime::new(100)));
    assert_ne!(first, ProcessKey::new(pid, Start::Unknown));
}

#[test]
fn start_times_from_one_source_compare() {
    assert!(StartTime::new(100) < StartTime::new(101));
    assert_eq!(StartTime::new(7), StartTime::new(7));
}

#[test]
fn sessions_with_different_serials_differ() {
    assert_ne!(TaskSession::new(1), TaskSession::new(2));
    assert_eq!(TaskSession::new(3), TaskSession::new(3));
}
