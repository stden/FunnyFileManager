//! Purpose: Unit tests for ambiguous/nonexistent DST wall times and unconvertible timestamps.

use super::{parse_time_in, to_system_time, InvalidTime};

#[test]
fn dst_gap_and_fold_times_are_refused() {
    let eastern = jiff::tz::TimeZone::posix("EST5EDT,M3.2.0,M11.1.0").expect("valid posix tz");
    for text in ["2024-03-10 02:30:00", "2024-11-03 01:30:00"] {
        let error = parse_time_in("Modification time", text, &eastern, to_system_time)
            .expect_err("ambiguous or nonexistent local time");
        assert_eq!(
            error,
            InvalidTime {
                field: "Modification time",
                text: text.to_owned(),
            }
        );
    }
}

#[test]
fn unconvertible_system_timestamp_is_refused() {
    let utc = jiff::tz::TimeZone::UTC;
    let error = parse_time_in("Access time", "2024-06-01 12:00:00", &utc, |_| None)
        .expect_err("unconvertible timestamp");
    assert_eq!(
        error,
        InvalidTime {
            field: "Access time",
            text: "2024-06-01 12:00:00".to_owned(),
        }
    );
}
