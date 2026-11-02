//! Unit tests for pure Windows conversion helpers (`windows_parse`).

use std::time::Duration;

use crate::models::tasks::{AdapterKind, InterfaceState};
use crate::models::{Niceness, Priority, PriorityClass, SettableClass};
use crate::platform::windows_parse::{
    classify_adapter, filetime_ticks, memory_and_swap, oper_status_state, priority_from_win32,
    settable_from_nice, ticks_to_duration, wide_buf_to_string, win32_from_settable,
};

#[test]
fn filetime_ticks_and_duration_convert_100ns_units_without_overflow() {
    let ticks = filetime_ticks(10_000_005, 0);
    assert_eq!(ticks, 10_000_005);
    assert_eq!(ticks_to_duration(ticks), Duration::new(1, 500));

    let high = filetime_ticks(0, 1);
    assert_eq!(high, 1_u64 << 32);
    assert!(ticks_to_duration(high) > Duration::from_secs(429));
}

#[test]
fn wide_buf_stops_at_first_nul_and_handles_full_buffers() {
    let padded = [
        u16::from(b'f'),
        u16::from(b'f'),
        u16::from(b'm'),
        0,
        u16::from(b'x'),
    ];
    assert_eq!(wide_buf_to_string(&padded), "ffm");

    let unpadded = [u16::from(b'a'), u16::from(b'b')];
    assert_eq!(wide_buf_to_string(&unpadded), "ab");
}

#[test]
fn priority_classes_round_trip_with_win32_flags_and_nice_buckets() {
    for settable in [
        SettableClass::Idle,
        SettableClass::BelowNormal,
        SettableClass::Normal,
        SettableClass::AboveNormal,
        SettableClass::High,
    ] {
        let raw = win32_from_settable(settable);
        assert_eq!(
            priority_from_win32(raw),
            Priority::Windows {
                class: settable.as_class()
            }
        );
    }
    assert_eq!(
        priority_from_win32(0x100),
        Priority::Windows {
            class: PriorityClass::Realtime
        }
    );
    assert_eq!(priority_from_win32(0), Priority::Unknown);

    assert_eq!(settable_from_nice(Niceness::MIN), SettableClass::High);
    assert_eq!(settable_from_nice(Niceness::NORMAL), SettableClass::Normal);
    assert_eq!(settable_from_nice(Niceness::MAX), SettableClass::Idle);
}

#[test]
fn memory_and_swap_separate_commit_limit_into_ram_and_pagefile() {
    let gib = 1024 * 1024 * 1024_u64;
    let (mem, swap) = memory_and_swap(16 * gib, 6 * gib, 20 * gib, 9 * gib);
    assert_eq!(mem.total.bytes(), 16 * gib);
    assert_eq!(mem.free.bytes(), 6 * gib);
    assert_eq!(swap.total.bytes(), 4 * gib);
    assert_eq!(swap.free.bytes(), 3 * gib);
}

#[test]
fn classify_adapter_and_oper_status_handle_physical_virtual_and_loopback() {
    assert_eq!(
        classify_adapter(24, "lo", "Software Loopback"),
        AdapterKind::Loopback
    );
    assert_eq!(
        classify_adapter(6, "vEthernet (WSL)", "Hyper-V Virtual Ethernet Adapter"),
        AdapterKind::Virtual
    );
    assert_eq!(
        classify_adapter(6, "Ethernet", "Intel I225-V"),
        AdapterKind::Ethernet
    );
    assert_eq!(
        classify_adapter(71, "Wi-Fi", "Intel AX210"),
        AdapterKind::Wifi
    );
    assert_eq!(classify_adapter(131, "tun0", "Tunnel"), AdapterKind::Other);

    assert_eq!(oper_status_state(1), InterfaceState::Up);
    assert_eq!(oper_status_state(2), InterfaceState::Down);
    assert_eq!(oper_status_state(5), InterfaceState::Dormant);
    assert_eq!(oper_status_state(99), InterfaceState::Unknown);
}
