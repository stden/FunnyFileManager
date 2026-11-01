//! Purpose: Windows process owner lookup (cached by SID) and identity-checked
//! process termination and priority-class updates.

use std::collections::HashMap;
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};

use windows_sys::Win32::Foundation::{HANDLE, STILL_ACTIVE};
use windows_sys::Win32::Security::{
    GetLengthSid, GetTokenInformation, IsValidSid, LookupAccountSidW, TokenUser, SID_NAME_USE,
    TOKEN_QUERY, TOKEN_USER,
};
use windows_sys::Win32::System::Threading::{
    GetExitCodeProcess, IsProcessCritical, OpenProcess, OpenProcessToken, SetPriorityClass,
    TerminateProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_INFORMATION,
    PROCESS_TERMINATE,
};

use super::process_build::{map_win32_error, validate_priority, validate_terminate};
use super::processes::{open_limited, query_times};
use crate::models::{
    ControlError, Owner, Pid, PriorityChange, ProcessKey, Signal, Start, StartTime,
};

/// Reads the account name owning `process`, caching `LookupAccountSidW` by SID.
pub(super) fn read_process_owner(process: HANDLE, cache: &mut HashMap<Vec<u8>, String>) -> Owner {
    let Some(token) = open_token(process) else {
        return Owner::Unknown;
    };
    let Some(sid_bytes) = token_user_sid(token.as_raw_handle()) else {
        return Owner::Unknown;
    };
    if let Some(cached) = cache.get(&sid_bytes) {
        return Owner::Named(cached.clone());
    }
    let Some(name) = lookup_sid_name(&sid_bytes) else {
        return Owner::Unknown;
    };
    cache.insert(sid_bytes, name.clone());
    Owner::Named(name)
}

#[allow(unsafe_code, reason = "Win32 OpenProcessToken query")]
fn open_token(process: HANDLE) -> Option<OwnedHandle> {
    let mut raw: HANDLE = std::ptr::null_mut();
    // SAFETY: `process` is valid and `raw` receives the opened token handle.
    if unsafe { OpenProcessToken(process, TOKEN_QUERY, &raw mut raw) } == 0 || raw.is_null() {
        return None;
    }
    // SAFETY: `raw` is a fresh token handle owned solely by the caller.
    Some(unsafe { OwnedHandle::from_raw_handle(raw) })
}

#[allow(unsafe_code, reason = "Win32 GetTokenInformation SID read")]
fn token_user_sid(token: HANDLE) -> Option<Vec<u8>> {
    let (mut buf, mut needed) = ([0_u8; 128], 0_u32);
    let cap = u32::try_from(buf.len()).unwrap_or(0);
    let ptr = buf.as_mut_ptr().cast();
    // SAFETY: `ptr` addresses `cap` writable bytes on stack; `needed` is writable.
    if unsafe { GetTokenInformation(token, TokenUser, ptr, cap, &raw mut needed) } == 0 {
        return None;
    }
    // SAFETY: `GetTokenInformation` wrote a `TOKEN_USER` at the start of `buf`.
    let psid = unsafe { buf.as_ptr().cast::<TOKEN_USER>().read_unaligned() }
        .User
        .Sid;
    // SAFETY: `psid` points inside `buf` and was populated by `GetTokenInformation`.
    if psid.is_null() || unsafe { IsValidSid(psid) } == 0 {
        return None;
    }
    // SAFETY: `psid` was validated by `IsValidSid`.
    let sid_len = usize::try_from(unsafe { GetLengthSid(psid) }).ok()?;
    (sid_len > 0 && sid_len <= buf.len()).then(|| {
        // SAFETY: `psid` addresses `sid_len` initialized bytes inside `buf`.
        unsafe { std::slice::from_raw_parts(psid.cast::<u8>(), sid_len) }.to_vec()
    })
}

#[allow(unsafe_code, reason = "Win32 LookupAccountSidW account lookup")]
fn lookup_sid_name(sid: &[u8]) -> Option<String> {
    let (mut name, mut domain) = (vec![0_u16; 128], vec![0_u16; 128]);
    let (mut name_len, mut dom_len) = (128_u32, 128_u32);
    let mut kind: SID_NAME_USE = 0;
    let psid = sid.as_ptr().cast_mut().cast();
    // SAFETY: `psid` is a validated SID copy; `name` and `domain` are 128-u16 writable buffers.
    let ok = unsafe {
        LookupAccountSidW(
            std::ptr::null(),
            psid,
            name.as_mut_ptr(),
            &raw mut name_len,
            domain.as_mut_ptr(),
            &raw mut dom_len,
            &raw mut kind,
        )
    };
    let n = usize::try_from(name_len).ok()?;
    (ok != 0 && n > 0).then(|| String::from_utf16_lossy(name.get(..n).unwrap_or(&[])))
}

/// Terminates `key` only after pinning a handle and confirming its `StartTime`.
pub fn terminate_process(key: ProcessKey, signal: Signal, own: Pid) -> Result<(), ControlError> {
    let (pid, expected) = validate_terminate(key, signal, own)?;
    let handle = open_verified(pid, expected, PROCESS_TERMINATE)?;
    if is_critical(handle.as_raw_handle()) {
        return Err(ControlError::Critical);
    }
    do_terminate(handle.as_raw_handle())
}

/// Updates the priority class of `key` after confirming its `StartTime`.
pub fn set_process_priority(key: ProcessKey, change: PriorityChange) -> Result<(), ControlError> {
    let (pid, expected, flag) = validate_priority(key, change)?;
    let handle = open_verified(pid, expected, PROCESS_SET_INFORMATION)?;
    do_set_priority(handle.as_raw_handle(), flag)
}

#[allow(unsafe_code, reason = "Win32 OpenProcess handle pinning")]
fn open_verified(pid: Pid, expected: StartTime, rights: u32) -> Result<OwnedHandle, ControlError> {
    if let Some(ro) = open_limited(pid.get()) {
        check_handle_identity(ro.as_raw_handle(), expected)?;
    }
    // SAFETY: `OpenProcess` requests `PROCESS_QUERY_LIMITED_INFORMATION | rights`.
    let raw = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | rights, 0, pid.get()) };
    if raw.is_null() {
        return Err(map_win32_error(&io::Error::last_os_error()));
    }
    // SAFETY: `raw` is a fresh process handle owned solely by `handle`.
    let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
    check_handle_identity(handle.as_raw_handle(), expected)?;
    Ok(handle)
}

fn check_handle_identity(handle: HANDLE, expected: StartTime) -> Result<(), ControlError> {
    let Some((Start::Known(actual), _, _)) = query_times(handle, 0) else {
        return Err(ControlError::Unverifiable);
    };
    if actual != expected {
        return Err(ControlError::Replaced);
    }
    still_active(handle).then_some(()).ok_or(ControlError::Gone)
}

#[allow(unsafe_code, reason = "Win32 GetExitCodeProcess liveness check")]
fn still_active(handle: HANDLE) -> bool {
    let mut code = 0_u32;
    // SAFETY: `handle` has limited query rights and `code` is writable.
    let ok = unsafe { GetExitCodeProcess(handle, &raw mut code) };
    ok != 0 && i32::try_from(code) == Ok(STILL_ACTIVE)
}

#[allow(unsafe_code, reason = "Win32 IsProcessCritical check")]
fn is_critical(handle: HANDLE) -> bool {
    let mut crit = 0_i32;
    // SAFETY: `handle` has query rights and `crit` is writable.
    unsafe { IsProcessCritical(handle, &raw mut crit) != 0 && crit != 0 }
}

#[allow(unsafe_code, reason = "Win32 TerminateProcess verified handle")]
fn do_terminate(handle: HANDLE) -> Result<(), ControlError> {
    // SAFETY: `handle` has `PROCESS_TERMINATE` and verified `StartTime`.
    if unsafe { TerminateProcess(handle, 1) } != 0 || !still_active(handle) {
        return Ok(());
    }
    Err(map_win32_error(&io::Error::last_os_error()))
}

#[allow(unsafe_code, reason = "Win32 SetPriorityClass verified handle")]
fn do_set_priority(handle: HANDLE, class_flag: u32) -> Result<(), ControlError> {
    // SAFETY: `handle` has `PROCESS_SET_INFORMATION` and verified `StartTime`.
    if unsafe { SetPriorityClass(handle, class_flag) } != 0 {
        return Ok(());
    }
    Err(map_win32_error(&io::Error::last_os_error()))
}
