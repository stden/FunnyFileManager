use super::{format, now};
use chrono::{Local, Timelike};

#[test]
fn midnight_and_noon_are_written_twelve_not_zero() {
    assert_eq!(format(0), "12:00 AM");
    assert_eq!(format(12 * 3600), "12:00 PM");
}

#[test]
fn the_afternoon_counts_from_one_again() {
    assert_eq!(format(13 * 3600 + 5 * 60), " 1:05 PM");
    assert_eq!(format(23 * 3600 + 43 * 60), "11:43 PM");
}

#[test]
fn the_morning_reads_as_it_should() {
    assert_eq!(format(9 * 3600 + 7 * 60), " 9:07 AM");
    assert_eq!(format(11 * 3600 + 59 * 60), "11:59 AM");
}

/// Seconds never show: the clock is glanced at, and a ticking seconds field
/// redraws the bar sixty times a minute for no one.
#[test]
fn seconds_do_not_change_the_reading() {
    assert_eq!(format(9 * 3600 + 7 * 60), format(9 * 3600 + 7 * 60 + 59));
}

#[test]
fn every_minute_of_the_day_formats() {
    for minute in 0..24 * 60u64 {
        let text = format(minute * 60);
        assert_eq!(text.chars().count(), 8, "{text:?}");
        assert!(text.ends_with(" AM") || text.ends_with(" PM"), "{text:?}");
    }
}

/// The whole point of the module: the corner of the bar reads the *local*
/// hour. Compared against chrono directly rather than against a fixed string,
/// because the only thing worth pinning is that the two agree — on Windows
/// they did not, and the clock silently showed UTC.
///
/// This can only fail on a machine whose offset is nonzero: at offset zero
/// every chrono-derived reading collapses into the UTC one, so no assertion
/// built from chrono alone can see the fallback there. That is why there is
/// no separate offset-pinning test — one existed, and it held for every
/// offset by chrono's own definition of local time.
#[test]
fn the_clock_reads_local_time_not_utc() {
    super::set_test_time(None);
    let at = Local::now();
    let expected = format(u64::from(at.num_seconds_from_midnight()));
    // A minute may tick over between the two reads; either side is correct.
    let a_minute_later = format(u64::from(at.num_seconds_from_midnight()) + 60);
    let actual = now();
    assert!(
        actual == expected || actual == a_minute_later,
        "clock said {actual:?}, local time is {expected:?}"
    );
}
