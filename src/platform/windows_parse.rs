//! Purpose: Pure, cross-platform conversions for Windows process, memory and
//! network raw structures so CI tests every edge case without `unsafe`.

use std::time::Duration;

use crate::models::tasks::{AdapterKind, InterfaceState};
use crate::models::{
    MemoryBytes, MemorySample, Niceness, Priority, PriorityClass, SettableClass, SwapSample,
};

const TICKS_PER_SEC: u64 = 10_000_000;
const NANOS_PER_TICK: u64 = 100;

const IDLE_CLASS: u32 = 0x0000_0040;
const BELOW_NORMAL_CLASS: u32 = 0x0000_4000;
const NORMAL_CLASS: u32 = 0x0000_0020;
const ABOVE_NORMAL_CLASS: u32 = 0x0000_8000;
const HIGH_CLASS: u32 = 0x0000_0080;
const REALTIME_CLASS: u32 = 0x0000_0100;

const IF_TYPE_ETHERNET: u32 = 6;
const IF_TYPE_LOOPBACK: u32 = 24;
const IF_TYPE_WIFI: u32 = 71;

/// Combines a Win32 `FILETIME`'s low and high 32-bit words into 100-ns ticks.
#[must_use]
pub fn filetime_ticks(low: u32, high: u32) -> u64 {
    (u64::from(high) << 32) | u64::from(low)
}

/// Converts 100-ns `FILETIME` ticks into a `Duration`.
#[must_use]
pub fn ticks_to_duration(ticks: u64) -> Duration {
    let secs = ticks / TICKS_PER_SEC;
    let rem_nanos = (ticks % TICKS_PER_SEC).saturating_mul(NANOS_PER_TICK);
    let nanos = u32::try_from(rem_nanos).unwrap_or(0);
    Duration::new(secs, nanos)
}

/// Decodes a NUL-padded UTF-16 buffer into a `String`.
#[must_use]
pub fn wide_buf_to_string(buf: &[u16]) -> String {
    let units: Vec<u16> = buf.iter().copied().take_while(|&c| c != 0).collect();
    String::from_utf16_lossy(&units)
}

/// Maps a Win32 `GetPriorityClass` return value (0 on failure) to `Priority`.
#[must_use]
pub const fn priority_from_win32(raw: u32) -> Priority {
    let class = match raw {
        IDLE_CLASS => PriorityClass::Idle,
        BELOW_NORMAL_CLASS => PriorityClass::BelowNormal,
        NORMAL_CLASS => PriorityClass::Normal,
        ABOVE_NORMAL_CLASS => PriorityClass::AboveNormal,
        HIGH_CLASS => PriorityClass::High,
        REALTIME_CLASS => PriorityClass::Realtime,
        _ => return Priority::Unknown,
    };
    Priority::Windows { class }
}

/// Maps `SettableClass` to its Win32 `SetPriorityClass` flag.
#[must_use]
pub const fn win32_from_settable(class: SettableClass) -> u32 {
    match class {
        SettableClass::Idle => IDLE_CLASS,
        SettableClass::BelowNormal => BELOW_NORMAL_CLASS,
        SettableClass::Normal => NORMAL_CLASS,
        SettableClass::AboveNormal => ABOVE_NORMAL_CLASS,
        SettableClass::High => HIGH_CLASS,
    }
}

/// Maps a Unix `Niceness` value (from the `F7` Priority form) to the nearest
/// Windows `SettableClass`.
#[must_use]
pub const fn settable_from_nice(nice: Niceness) -> SettableClass {
    match nice.get() {
        ..=-10 => SettableClass::High,
        -9..=-1 => SettableClass::AboveNormal,
        0 => SettableClass::Normal,
        1..=9 => SettableClass::BelowNormal,
        10.. => SettableClass::Idle,
    }
}

/// Builds `MemorySample` and `SwapSample` from `MEMORYSTATUSEX` byte fields.
/// Note: Win32 `ullTotalPageFile` is commit limit (RAM + page files), so the
/// swap/pagefile portion is `total_page_file.saturating_sub(total_phys)`.
#[must_use]
pub const fn memory_and_swap(
    total_phys: u64,
    avail_phys: u64,
    total_page_file: u64,
    avail_page_file: u64,
) -> (MemorySample, SwapSample) {
    let zero = MemoryBytes::from_bytes(0);
    let mem_free = if avail_phys <= total_phys {
        avail_phys
    } else {
        total_phys
    };
    let memory = MemorySample {
        total: MemoryBytes::from_bytes(total_phys),
        free: MemoryBytes::from_bytes(mem_free),
        available: MemoryBytes::from_bytes(mem_free),
        buffers: zero,
        cached: zero,
        reclaimable: zero,
        shared: zero,
    };
    let swap_total = total_page_file.saturating_sub(total_phys);
    let used_commit = total_page_file.saturating_sub(avail_page_file);
    let used_phys = total_phys.saturating_sub(mem_free);
    let swap_used = used_commit.saturating_sub(used_phys);
    let swap_free = swap_total.saturating_sub(swap_used);
    let swap = SwapSample {
        total: MemoryBytes::from_bytes(swap_total),
        free: MemoryBytes::from_bytes(swap_free),
        cached: zero,
    };
    (memory, swap)
}

/// Classifies an NDIS interface from `MIB_IF_ROW2::Type` and description/alias.
#[must_use]
pub fn classify_adapter(if_type: u32, name: &str, description: &str) -> AdapterKind {
    if if_type == IF_TYPE_LOOPBACK {
        return AdapterKind::Loopback;
    }
    if is_virtual_name(name) || is_virtual_name(description) {
        return AdapterKind::Virtual;
    }
    match if_type {
        IF_TYPE_ETHERNET => AdapterKind::Ethernet,
        IF_TYPE_WIFI => AdapterKind::Wifi,
        _ => AdapterKind::Other,
    }
}

fn is_virtual_name(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "vethernet",
        "hyper-v",
        "virtual",
        "wsl",
        "wireguard",
        "tailscale",
        "tap-windows",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

/// Maps Win32 `IF_OPER_STATUS` (`1 = Up`, `2 = Down`, `5 = Dormant`).
#[must_use]
pub const fn oper_status_state(status: i32) -> InterfaceState {
    match status {
        1 => InterfaceState::Up,
        2 | 6 => InterfaceState::Down,
        5 => InterfaceState::Dormant,
        _ => InterfaceState::Unknown,
    }
}
