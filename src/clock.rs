//! Purpose: the time, as Far shows it in the corner of the menu bar.
//!
//! Formatted from the system clock at draw time. Keeping it in `App` would
//! mean a stored copy of something that is only ever true for a moment, and a
//! second thing to remember to update.
//!
//! Local time comes from `chrono`, which the crate already depends on for file
//! timestamps — so there is one way to ask what the local time is, not two.
//! The earlier version shelled out to `date +%z` on Unix and had no answer at
//! all on Windows, where the offset was zero and the corner of the screen
//! quietly showed UTC.

use chrono::{Local, Timelike};
#[cfg(test)]
use std::cell::Cell;

/// Seconds in an hour and a minute.
const HOUR: u32 = 3_600;
const MINUTE: u32 = 60;

#[cfg(test)]
thread_local! {
    /// The time unit tests read, in seconds since midnight: noon, standing
    /// still unless a test moves it. The event loop redraws when the minute
    /// turns over, and a minute that happened to turn over mid-test would
    /// add a frame to every test that counts them. `None` reads the real clock.
    static TEST_TIME: Cell<Option<u64>> = const { Cell::new(Some(12 * 3_600)) };
}

/// Set the time [`now`] reads on this test's thread; `None` for the real clock.
#[cfg(test)]
pub(crate) fn set_test_time(since_midnight: Option<u64>) {
    TEST_TIME.with(|time| time.set(since_midnight));
}

/// The current local time as Far writes it: `11:43 PM`.
#[must_use]
pub fn now() -> String {
    #[cfg(test)]
    if let Some(frozen) = TEST_TIME.with(Cell::get) {
        return format(frozen);
    }
    format(u64::from(Local::now().num_seconds_from_midnight()))
}

/// Twelve-hour clock with the meridiem, which is the shape Far uses.
fn format(since_midnight: u64) -> String {
    let hour24 = (since_midnight / u64::from(HOUR)) % 24;
    let minute = (since_midnight % u64::from(HOUR)) / u64::from(MINUTE);
    let meridiem = if hour24 < 12 { "AM" } else { "PM" };
    // Midnight and noon are both written 12, not 0.
    let hour = match hour24 % 12 {
        0 => 12,
        other => other,
    };
    format!("{hour:2}:{minute:02} {meridiem}")
}

#[cfg(test)]
mod tests;
