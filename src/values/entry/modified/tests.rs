//! Purpose: Unit tests for `Modified` date/time/stamp formatting and local-minute memoization.

use std::time::{Duration, UNIX_EPOCH};

use super::Modified;

fn reference_strings(at: std::time::SystemTime) -> (String, String, String) {
    let dt = jiff::Timestamp::try_from(at)
        .expect("sample instant")
        .to_zoned_datetime(jiff::tz::TimeZone::system());
    let date = format!(
        "{:02}/{:02}/{:02}",
        dt.month(),
        dt.day(),
        dt.year().rem_euclid(100)
    );
    let time = format!("{:02}:{:02}", dt.hour(), dt.minute());
    let stamp = format!("{date} {time}");
    (date, time, stamp)
}

#[test]
fn unknown_and_epoch_boundaries_match_local_civil_time() {
    assert_eq!(Modified::UNKNOWN.date(), "");
    assert_eq!(Modified::UNKNOWN.time(), "");
    assert_eq!(Modified::UNKNOWN.stamp(), "");

    let samples = [
        UNIX_EPOCH - Duration::from_secs(7_200),
        UNIX_EPOCH - Duration::from_millis(500),
        UNIX_EPOCH,
        UNIX_EPOCH + Duration::from_secs(1_700_000_000),
        UNIX_EPOCH + Duration::from_secs(1_700_000_045),
    ];
    for raw in samples {
        let modified = Modified::new(raw);
        let (date, time, stamp) = reference_strings(raw);
        assert_eq!(modified.date(), date);
        assert_eq!(modified.time(), time);
        assert_eq!(modified.stamp(), stamp);
    }

    for minute in 0_u64..260 {
        let raw = UNIX_EPOCH + Duration::from_secs(1_710_000_000 + minute * 60);
        let modified = Modified::new(raw);
        let (date, time, stamp) = reference_strings(raw);
        assert_eq!(modified.date(), date);
        assert_eq!(modified.time(), time);
        assert_eq!(modified.stamp(), stamp);
    }
}

/// One second past the last instant jiff can name. A zero date would paint
/// `00/00/00` in the column; unknown is blank.
#[test]
fn an_instant_past_the_calendar_is_blank() {
    let past = std::time::SystemTime::try_from(jiff::Timestamp::MAX).expect("year 9999")
        + Duration::from_secs(1);
    assert!(jiff::Timestamp::try_from(past).is_err());
    let modified = Modified::new(past);
    assert_eq!(modified.date(), "");
    assert_eq!(modified.time(), "");
    assert_eq!(modified.stamp(), "");
}
