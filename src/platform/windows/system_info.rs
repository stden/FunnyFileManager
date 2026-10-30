//! Purpose: Reads Windows system-wide CPU counters, RAM/pagefile sizes, and
//! uptime through `GetSystemTimes`, `GlobalMemoryStatusEx` and `GetTickCount64`.

use std::io;
use std::time::Duration;

use windows_sys::Win32::Foundation::FILETIME;
use windows_sys::Win32::System::SystemInformation::{
    GetSystemTimeAsFileTime, GetTickCount64, GlobalMemoryStatusEx, MEMORYSTATUSEX,
};
use windows_sys::Win32::System::Threading::GetSystemTimes;

use crate::models::{CoreSample, CpuCounters, MemorySample, SwapSample, SystemSample};
use crate::platform::windows_parse::{filetime_ticks, memory_and_swap, ticks_to_duration};

/// Current UTC wall-clock stamp in 100-ns `FILETIME` ticks, used to compute
/// how long each running process has been alive (`TIME+` / `Elapsed`).
#[allow(
    unsafe_code,
    reason = "GetSystemTimeAsFileTime matches GetProcessTimes units"
)]
#[must_use]
pub fn now_filetime_ticks() -> u64 {
    let mut now = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    // SAFETY: `now` is a valid writable `FILETIME` on the stack.
    unsafe { GetSystemTimeAsFileTime(&raw mut now) };
    filetime_ticks(now.dwLowDateTime, now.dwHighDateTime)
}

/// Reads system-wide CPU counters, logical core count, memory, swap and uptime.
pub fn read_system_sample() -> io::Result<SystemSample> {
    let (idle, kernel, user) = raw_system_times()?;
    // On Windows `lpKernelTime` includes `lpIdleTime`.
    let system_ticks = kernel.saturating_sub(idle);
    let total = CpuCounters {
        user: ticks_to_duration(user),
        nice: Duration::ZERO,
        system: ticks_to_duration(system_ticks),
        idle: ticks_to_duration(idle),
        virtualised: Duration::ZERO,
    };
    let cores = split_across_cores(idle, system_ticks, user);
    let (memory, swap) = read_memory().map_or((None, None), |(m, s)| (Some(m), Some(s)));
    Ok(SystemSample {
        total,
        cores,
        memory,
        swap,
        load: None,
        uptime: Some(Duration::from_millis(uptime_millis())),
        running: None,
    })
}

#[allow(unsafe_code, reason = "std exposes no system-wide CPU counter API")]
fn raw_system_times() -> io::Result<(u64, u64, u64)> {
    let zero = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let (mut idle, mut kernel, mut user) = (zero, zero, zero);
    // SAFETY: all three pointers address valid writable `FILETIME` locals.
    let ok = unsafe { GetSystemTimes(&raw mut idle, &raw mut kernel, &raw mut user) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((
        filetime_ticks(idle.dwLowDateTime, idle.dwHighDateTime),
        filetime_ticks(kernel.dwLowDateTime, kernel.dwHighDateTime),
        filetime_ticks(user.dwLowDateTime, user.dwHighDateTime),
    ))
}

fn split_across_cores(idle: u64, system: u64, user: u64) -> Vec<CoreSample> {
    let count = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let n = u64::try_from(count).unwrap_or(1).max(1);
    let per_core = CpuCounters {
        user: ticks_to_duration(user / n),
        nice: Duration::ZERO,
        system: ticks_to_duration(system / n),
        idle: ticks_to_duration(idle / n),
        virtualised: Duration::ZERO,
    };
    let limit = u16::try_from(count).unwrap_or(u16::MAX);
    (0..limit)
        .map(|index| CoreSample {
            index,
            counters: per_core,
        })
        .collect()
}

#[allow(unsafe_code, reason = "std exposes no physical RAM or pagefile query")]
fn read_memory() -> Option<(MemorySample, SwapSample)> {
    let mut status = MEMORYSTATUSEX {
        dwLength: u32::try_from(size_of::<MEMORYSTATUSEX>()).unwrap_or(0),
        ..MEMORYSTATUSEX::default()
    };
    // SAFETY: `status` is initialized with `dwLength = sizeof(MEMORYSTATUSEX)`.
    let ok = unsafe { GlobalMemoryStatusEx(&raw mut status) };
    (ok != 0).then(|| {
        memory_and_swap(
            status.ullTotalPhys,
            status.ullAvailPhys,
            status.ullTotalPageFile,
            status.ullAvailPageFile,
        )
    })
}

#[allow(unsafe_code, reason = "std exposes no monotonic boot uptime")]
fn uptime_millis() -> u64 {
    // SAFETY: `GetTickCount64` takes no arguments and reads no memory.
    unsafe { GetTickCount64() }
}
