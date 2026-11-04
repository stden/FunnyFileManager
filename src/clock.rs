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
use std::cell::RefCell;
use std::time::{SystemTime, UNIX_EPOCH};

/// Time units used for the 12-hour wall-clock display.
const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 3_600;
const HOURS_PER_DAY: u64 = 24;
const HALF_DAY_HOURS: u64 = 12;

struct CachedReading {
    epoch_sec: u64,
    formatted: String,
}

thread_local! {
    static REAL_CACHE: RefCell<Option<CachedReading>> = const { RefCell::new(None) };
}

#[cfg(test)]
thread_local! {
    /// The time unit tests read, in seconds since midnight: noon, standing
    /// still unless a test moves it. The event loop redraws when the minute
    /// turns over, and a minute that happened to turn over mid-test would
    /// add a frame to every test that counts them. `None` reads the real clock.
    static TEST_TIME: Cell<Option<u64>> = const { Cell::new(Some(HALF_DAY_HOURS * SECONDS_PER_HOUR)) };
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
    let epoch_sec = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    cached_at(epoch_sec, || {
        u64::from(Local::now().num_seconds_from_midnight())
    })
}

fn cached_at(epoch_sec: u64, since_midnight: impl FnOnce() -> u64) -> String {
    REAL_CACHE.with(|slot| {
        if let Some(cached) = slot.borrow().as_ref() {
            if cached.epoch_sec == epoch_sec {
                return cached.formatted.clone();
            }
        }
        let formatted = format(since_midnight());
        *slot.borrow_mut() = Some(CachedReading {
            epoch_sec,
            formatted: formatted.clone(),
        });
        formatted
    })
}

/// Twelve-hour clock with the meridiem, which is the shape Far uses.
fn format(since_midnight: u64) -> String {
    let hour24 = (since_midnight / SECONDS_PER_HOUR) % HOURS_PER_DAY;
    let minute = (since_midnight % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE;
    let meridiem = if hour24 < HALF_DAY_HOURS { "AM" } else { "PM" };
    // Midnight and noon are both written 12, not 0.
    let hour12 = match hour24 % HALF_DAY_HOURS {
        0 => HALF_DAY_HOURS,
        other => other,
    };
    format!("{hour12:2}:{minute:02} {meridiem}")
}

#[cfg(test)]
mod tests;
