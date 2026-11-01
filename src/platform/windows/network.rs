//! Purpose: Reads Windows network adapter counters via `GetIfTable2Ex` and
//! sends unprivileged ICMP echo probes via `IcmpSendEcho`.

use std::collections::HashMap;
use std::mem::offset_of;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    FreeMibTable, GetIfTable2Ex, IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho, MibIfTableNormal,
    ICMP_ECHO_REPLY, MIB_IF_ROW2, MIB_IF_TABLE2,
};

use crate::models::tasks::{AdapterSample, Ipv4, ProbeOutcome};
use crate::platform::windows_parse::{classify_adapter, oper_status_state, wide_buf_to_string};

const FILTER_INTERFACE_BIT: u8 = 0x02;
const IP_REQ_TIMED_OUT: u32 = 11_010;

struct MibTable(*mut MIB_IF_TABLE2);

impl Drop for MibTable {
    #[allow(unsafe_code, reason = "FreeMibTable releases GetIfTable2Ex memory")]
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: `self.0` was allocated by `GetIfTable2Ex` and freed once here.
            unsafe { FreeMibTable(self.0.cast()) };
        }
    }
}

/// Reads every non-filter network interface and its cumulative traffic counters.
#[allow(
    unsafe_code,
    reason = "GetIfTable2Ex enumerates NDIS interface counters"
)]
#[must_use]
pub fn read_network_adapters(ipv4_by_index: &HashMap<u32, Ipv4>) -> Vec<AdapterSample> {
    let mut table_ptr: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
    // SAFETY: `table_ptr` receives the `MIB_IF_TABLE2` allocated by IP Helper.
    if unsafe { GetIfTable2Ex(MibIfTableNormal, &raw mut table_ptr) } != 0 || table_ptr.is_null() {
        return Vec::new();
    }
    let guard = MibTable(table_ptr);
    // SAFETY: `guard.0` is non-null and initialized by `GetIfTable2Ex`.
    let count = usize::try_from(unsafe { (*guard.0).NumEntries }).unwrap_or(0);
    // SAFETY: `Table` starts within the `MIB_IF_TABLE2` allocation of `guard.0`.
    let base = unsafe {
        guard
            .0
            .byte_add(offset_of!(MIB_IF_TABLE2, Table))
            .cast::<MIB_IF_ROW2>()
    };
    let mut out = Vec::with_capacity(count);
    for idx in 0..count {
        // SAFETY: `idx < count` stays within the `NumEntries` rows of `guard.0`.
        let row_ptr = unsafe { base.add(idx) };
        // SAFETY: `row_ptr` addresses an initialized `MIB_IF_ROW2`.
        let row = unsafe { *row_ptr };
        if let Some(sample) = adapter_from_row(&row, ipv4_by_index) {
            out.push(sample);
        }
    }
    out
}

fn adapter_from_row(
    row: &MIB_IF_ROW2,
    ipv4_by_index: &HashMap<u32, Ipv4>,
) -> Option<AdapterSample> {
    if (row.InterfaceAndOperStatusFlags._bitfield & FILTER_INTERFACE_BIT) != 0 {
        return None;
    }
    let (name, description) = row_names(row)?;
    let link_bps = (row.ReceiveLinkSpeed > 0 && row.ReceiveLinkSpeed < u64::MAX)
        .then_some(row.ReceiveLinkSpeed);
    Some(AdapterSample {
        kind: classify_adapter(row.Type, &name, &description),
        name,
        description,
        state: oper_status_state(row.OperStatus),
        ipv4: ipv4_by_index.get(&row.InterfaceIndex).copied(),
        link_bps,
        rx_bytes: row.InOctets,
        tx_bytes: row.OutOctets,
        rx_packets: row.InUcastPkts.saturating_add(row.InNUcastPkts),
        tx_packets: row.OutUcastPkts.saturating_add(row.OutNUcastPkts),
        rx_errors: row.InErrors,
        tx_errors: row.OutErrors,
        rx_drops: row.InDiscards,
        tx_drops: row.OutDiscards,
    })
}

fn row_names(row: &MIB_IF_ROW2) -> Option<(String, String)> {
    let alias = wide_buf_to_string(&row.Alias);
    let description = wide_buf_to_string(&row.Description);
    let name = if alias.is_empty() {
        description.clone()
    } else {
        alias
    };
    (!name.is_empty()).then_some((name, description))
}

struct IcmpHandle(HANDLE);

impl Drop for IcmpHandle {
    #[allow(unsafe_code, reason = "IcmpCloseHandle closes IcmpCreateFile handles")]
    fn drop(&mut self) {
        // SAFETY: `self.0` was opened by `IcmpCreateFile` and is closed once.
        let _ = unsafe { IcmpCloseHandle(self.0) };
    }
}

/// Sends one unprivileged `ICMPv4` echo request to `target`.
#[allow(
    unsafe_code,
    reason = "IcmpSendEcho performs unprivileged ICMP echo on Windows"
)]
#[must_use]
pub fn icmp_ping(target: Ipv4, timeout: Duration) -> ProbeOutcome {
    // SAFETY: `IcmpCreateFile` takes no parameters.
    let raw = unsafe { IcmpCreateFile() };
    if raw == INVALID_HANDLE_VALUE || raw.is_null() {
        return ProbeOutcome::Unreachable;
    }
    let handle = IcmpHandle(raw);
    let mut reply_buf = [0_u64; 16];
    let payload = *b"ffm0";
    let reply_size = u32::try_from(size_of_val(&reply_buf)).unwrap_or(0);
    let timeout_ms = u32::try_from(timeout.as_millis())
        .unwrap_or(u32::MAX)
        .max(50);
    let start = Instant::now();
    // SAFETY: `handle.0` is valid, `payload` is 4 bytes, and `reply_buf` is 128 writable bytes.
    let count = unsafe {
        IcmpSendEcho(
            handle.0,
            u32::from_ne_bytes(target.octets()),
            payload.as_ptr().cast(),
            4,
            std::ptr::null(),
            reply_buf.as_mut_ptr().cast(),
            reply_size,
            timeout_ms,
        )
    };
    decode_icmp_reply(count, &reply_buf, start.elapsed())
}

#[allow(
    unsafe_code,
    reason = "Reads ICMP_ECHO_REPLY populated by IcmpSendEcho"
)]
fn decode_icmp_reply(count: u32, reply_buf: &[u64; 16], wall: Duration) -> ProbeOutcome {
    if count == 0 {
        let code = std::io::Error::last_os_error()
            .raw_os_error()
            .and_then(|c| u32::try_from(c).ok());
        return if code == Some(IP_REQ_TIMED_OUT) {
            ProbeOutcome::Timeout
        } else {
            ProbeOutcome::Unreachable
        };
    }
    // SAFETY: `count > 0` guarantees an initialized `ICMP_ECHO_REPLY` at `reply_buf`.
    let reply = unsafe {
        reply_buf
            .as_ptr()
            .cast::<ICMP_ECHO_REPLY>()
            .read_unaligned()
    };
    match reply.Status {
        0 => {
            let reported = Duration::from_millis(u64::from(reply.RoundTripTime));
            ProbeOutcome::Ok(reported.max(wall.min(Duration::from_millis(1))))
        }
        IP_REQ_TIMED_OUT => ProbeOutcome::Timeout,
        _ => ProbeOutcome::Unreachable,
    }
}
