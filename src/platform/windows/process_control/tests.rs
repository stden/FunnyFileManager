//! Purpose: Unit tests for null-handle, invalid-SID, and vanished-PID fallbacks in process control.

use super::*;

#[test]
fn owner_and_control_helpers_reject_null_handles_invalid_sids_and_missing_pids() {
    let mut cache = HashMap::new();
    assert_eq!(
        read_process_owner(std::ptr::null_mut(), &mut cache),
        Owner::Unknown
    );
    assert_eq!(token_user_sid(std::ptr::null_mut()), None);
    assert_eq!(lookup_sid_name(&[0_u8; 8]), None);
    assert_eq!(
        check_handle_identity(std::ptr::null_mut(), StartTime::new(1)),
        Err(ControlError::Unverifiable)
    );
    assert!(do_terminate(std::ptr::null_mut()).is_err());
    assert!(do_set_priority(std::ptr::null_mut(), 0).is_err());

    let bogus = ProcessKey::new(
        Pid::new(u32::MAX - 3).unwrap(),
        Start::Known(StartTime::new(1)),
    );
    let idle = PriorityChange::Class(crate::models::tasks::SettableClass::Idle);
    assert!(set_process_priority(bogus, idle).is_err());
}
