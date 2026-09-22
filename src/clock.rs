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

/// Seconds in an hour and a minute.
const HOUR: u32 = 3_600;
const MINUTE: u32 = 60;

/// The current local time as Far writes it: `11:43 PM`.
#[must_use]
pub fn now() -> String {
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
