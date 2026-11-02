//! Purpose: Unit tests for `MIB_IF_ROW2` conversion and `ICMP_ECHO_REPLY` status decoding.

use super::*;

#[test]
fn adapter_from_row_filters_flag_bits_and_falls_back_to_description() {
    let ips = HashMap::new();
    let mut filtered = MIB_IF_ROW2::default();
    filtered.InterfaceAndOperStatusFlags._bitfield = FILTER_INTERFACE_BIT;
    assert!(adapter_from_row(&filtered, &ips).is_none());

    let empty = MIB_IF_ROW2::default();
    assert!(adapter_from_row(&empty, &ips).is_none());

    let mut desc_only = MIB_IF_ROW2::default();
    for (dst, ch) in desc_only
        .Description
        .iter_mut()
        .zip("Realtek NIC".encode_utf16())
    {
        *dst = ch;
    }
    desc_only.ReceiveLinkSpeed = 1_000_000_000;
    let sample = adapter_from_row(&desc_only, &ips).unwrap();
    assert_eq!(sample.name, "Realtek NIC");
    assert_eq!(sample.link_bps, Some(1_000_000_000));
}

#[test]
fn decode_icmp_reply_maps_zero_count_timeout_and_unreachable_statuses() {
    let zero_buf = [0_u64; 16];
    let zero_out = decode_icmp_reply(0, &zero_buf, Duration::from_millis(5));
    assert!(matches!(
        zero_out,
        ProbeOutcome::Unreachable | ProbeOutcome::Timeout
    ));

    let mut ok_buf = [0_u64; 16];
    assert!(matches!(
        decode_icmp_reply(1, &ok_buf, Duration::from_millis(3)),
        ProbeOutcome::Ok(_)
    ));

    ok_buf[0] = u64::from(IP_REQ_TIMED_OUT) << 32;
    assert_eq!(
        decode_icmp_reply(1, &ok_buf, Duration::from_millis(3)),
        ProbeOutcome::Timeout
    );

    ok_buf[0] = 11_003_u64 << 32;
    assert_eq!(
        decode_icmp_reply(1, &ok_buf, Duration::from_millis(3)),
        ProbeOutcome::Unreachable
    );
}
