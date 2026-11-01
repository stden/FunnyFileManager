//! Purpose: Enumerates running Windows processes via Toolhelp32 and queries
//! each accessible process's start time, CPU time, memory, priority and owner.

use std::collections::HashMap;
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::time::Duration;

use windows_sys::Win32::Foundation::{FILETIME, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows_sys::Win32::System::Threading::{
    GetPriorityClass, GetProcessTimes, OpenProcess, QueryFullProcessImageNameW,
    PROCESS_QUERY_LIMITED_INFORMATION,
};

use super::process_build::{build_sample, HandleDetails, WinOwnerCache};
use super::process_control::read_process_owner;
use super::system_info::now_filetime_ticks;
use crate::models::{MemoryBytes, MemoryUsage, Priority, ProcessSample, Start, StartTime};
use crate::platform::windows_parse::{filetime_ticks, priority_from_win32, ticks_to_duration};

/// Enumerates all processes in a Toolhelp snapshot.
pub fn list_processes(owners: &mut WinOwnerCache) -> io::Result<(Vec<ProcessSample>, u32)> {
    let snap = open_snapshot()?;
    let now_ticks = now_filetime_ticks();
    let mut entry = blank_entry();
    if !step_snapshot(snap.as_raw_handle(), &mut entry, true) {
        return Ok((Vec::new(), 0));
    }
    let mut out = Vec::with_capacity(256);
    loop {
        let details = open_limited(entry.th32ProcessID)
            .map(|h| inspect_handle(h.as_raw_handle(), now_ticks, &mut owners.by_sid))
            .unwrap_or_default();
        if let Some(sample) = build_sample(
            entry.th32ProcessID,
            entry.th32ParentProcessID,
            entry.cntThreads,
            &entry.szExeFile,
            details,
        ) {
            out.push(sample);
        }
        entry = blank_entry();
        if !step_snapshot(snap.as_raw_handle(), &mut entry, false) {
            break;
        }
    }
    Ok((out, 0))
}

fn blank_entry() -> PROCESSENTRY32W {
    PROCESSENTRY32W {
        dwSize: u32::try_from(size_of::<PROCESSENTRY32W>()).unwrap_or(0),
        ..PROCESSENTRY32W::default()
    }
}

#[allow(unsafe_code, reason = "Toolhelp32 snapshot requires Win32 FFI")]
fn open_snapshot() -> io::Result<OwnedHandle> {
    // SAFETY: `TH32CS_SNAPPROCESS` with pid 0 snapshots the system process list.
    let raw = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if raw == INVALID_HANDLE_VALUE || raw.is_null() {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `raw` is a fresh snapshot handle owned solely by the caller.
    Ok(unsafe { OwnedHandle::from_raw_handle(raw) })
}

#[allow(
    unsafe_code,
    reason = "Process32FirstW/NextW iterate a Toolhelp snapshot"
)]
fn step_snapshot(snap: HANDLE, entry: &mut PROCESSENTRY32W, first: bool) -> bool {
    if first {
        // SAFETY: `snap` is live and `entry` has `dwSize` initialized.
        unsafe { Process32FirstW(snap, &raw mut *entry) != 0 }
    } else {
        // SAFETY: `snap` is live and `entry` has `dwSize` initialized.
        unsafe { Process32NextW(snap, &raw mut *entry) != 0 }
    }
}

fn inspect_handle(
    raw: HANDLE,
    now_ticks: u64,
    sid_cache: &mut HashMap<Vec<u8>, String>,
) -> HandleDetails {
    let (start, cpu_time, running_for) =
        query_times(raw, now_ticks).unwrap_or((Start::Unknown, Duration::ZERO, None));
    HandleDetails {
        start,
        cpu_time,
        running_for,
        image_path: query_image_path(raw),
        memory: query_memory(raw),
        priority: query_priority(raw),
        owner: read_process_owner(raw, sid_cache),
    }
}

#[allow(
    unsafe_code,
    reason = "OpenProcess opens a limited query handle per PID"
)]
pub(super) fn open_limited(pid: u32) -> Option<OwnedHandle> {
    // SAFETY: `OpenProcess` with `PROCESS_QUERY_LIMITED_INFORMATION` is read-only.
    let raw = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    (!raw.is_null()).then(|| {
        // SAFETY: `raw` is a fresh process handle owned solely by the caller.
        unsafe { OwnedHandle::from_raw_handle(raw) }
    })
}

#[allow(unsafe_code, reason = "GetProcessTimes reads creation and CPU times")]
pub(super) fn query_times(
    handle: HANDLE,
    now_ticks: u64,
) -> Option<(Start, Duration, Option<Duration>)> {
    let z = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let (mut created, mut exited, mut kernel, mut user) = (z, z, z, z);
    // SAFETY: `handle` has limited query access; all 4 FILETIME pointers are valid.
    let ok = unsafe {
        GetProcessTimes(
            handle,
            &raw mut created,
            &raw mut exited,
            &raw mut kernel,
            &raw mut user,
        )
    };
    if ok == 0 {
        return None;
    }
    let start_ticks = filetime_ticks(created.dwLowDateTime, created.dwHighDateTime);
    let cpu_ticks = filetime_ticks(kernel.dwLowDateTime, kernel.dwHighDateTime)
        .saturating_add(filetime_ticks(user.dwLowDateTime, user.dwHighDateTime));
    let elapsed = (now_ticks >= start_ticks).then(|| ticks_to_duration(now_ticks - start_ticks));
    Some((
        Start::Known(StartTime::new(start_ticks)),
        ticks_to_duration(cpu_ticks),
        elapsed,
    ))
}

#[allow(
    unsafe_code,
    reason = "K32GetProcessMemoryInfo reads working set and commit"
)]
fn query_memory(handle: HANDLE) -> Option<MemoryUsage> {
    let mut mem = PROCESS_MEMORY_COUNTERS::default();
    let cb = u32::try_from(size_of::<PROCESS_MEMORY_COUNTERS>()).unwrap_or(0);
    // SAFETY: `handle` has limited query access; `mem` is `cb` writable bytes.
    if unsafe { K32GetProcessMemoryInfo(handle, &raw mut mem, cb) } == 0 {
        return None;
    }
    let resident = u64::try_from(mem.WorkingSetSize).unwrap_or(u64::MAX);
    let commit = u64::try_from(mem.PagefileUsage).unwrap_or(u64::MAX);
    Some(MemoryUsage {
        virtual_size: MemoryBytes::from_bytes(commit.max(resident)),
        resident: MemoryBytes::from_bytes(resident),
        shared: None,
    })
}

#[allow(
    unsafe_code,
    reason = "QueryFullProcessImageNameW reads the image path"
)]
fn query_image_path(handle: HANDLE) -> Option<String> {
    let mut buf = vec![0_u16; 1024];
    let mut len = u32::try_from(buf.len()).unwrap_or(0);
    // SAFETY: `buf` holds `len` writable UTF-16 code units; `len` receives count.
    let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &raw mut len) };
    let written = usize::try_from(len).ok()?;
    (ok != 0 && written > 0).then(|| String::from_utf16_lossy(buf.get(..written).unwrap_or(&[])))
}

#[allow(
    unsafe_code,
    reason = "GetPriorityClass reads the process priority class"
)]
fn query_priority(handle: HANDLE) -> Priority {
    // SAFETY: `handle` has `PROCESS_QUERY_LIMITED_INFORMATION`.
    priority_from_win32(unsafe { GetPriorityClass(handle) })
}
