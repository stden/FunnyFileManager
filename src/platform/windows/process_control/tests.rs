//! Purpose: Unit tests for null-handle, invalid-SID, and vanished-PID fallbacks in process control.

use super::*;

#[test]
fn owner_helpers_reject_null_handles_and_invalid_sids() {
    let mut cache = HashMap::new();
    assert_eq!(
        read_process_owner(std::ptr::null_mut(), &mut cache),
        Owner::Unknown
    );
    assert_eq!(
        resolve_owner_sid(None, &mut cache, lookup_sid_name),
        Owner::Unknown
    );
    assert_eq!(
        resolve_owner_sid(Some(vec![1, 2, 3]), &mut cache, |_| None),
        Owner::Unknown
    );
    assert_eq!(token_user_sid(std::ptr::null_mut()), None);
    assert_eq!(decode_token_user_sid(&[0_u8; 128]), None);
    assert_eq!(lookup_sid_name(&[0_u8; 8]), None);
}

#[test]
fn control_helpers_reject_null_handles_missing_pids_and_exited_children() {
    assert_eq!(
        terminate_verified(std::ptr::null_mut(), |_| true),
        Err(ControlError::Critical)
    );
    assert_eq!(
        check_handle_identity(std::ptr::null_mut(), StartTime::new(1)),
        Err(ControlError::Unverifiable)
    );
    assert!(do_terminate(std::ptr::null_mut()).is_err());
    assert!(do_set_priority(std::ptr::null_mut(), 0).is_err());
    let bogus = ProcessKey::new(
        Pid::new(2_147_483_647).unwrap(),
        Start::Known(StartTime::new(1)),
    );
    let idle = PriorityChange::Class(crate::models::tasks::SettableClass::Idle);
    assert!(set_process_priority(bogus, idle).is_err());
    let mut exited = std::process::Command::new("cmd.exe")
        .args(["/C", "exit", "0"])
        .spawn()
        .unwrap();
    exited.wait().unwrap();
    assert_eq!(do_terminate(exited.as_raw_handle()), Ok(()));
}
